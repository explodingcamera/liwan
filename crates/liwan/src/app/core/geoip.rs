use std::io::{self, Read};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use arc_swap::ArcSwapOption;
use futures_util::StreamExt;
use md5::Digest;
use rand::distr::{Alphanumeric, SampleString};
use tar_codec::{Archive, Member, MemberPayload, TarArchive};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_util::io::StreamReader;

const BASE_URL: &str = "https://updates.maxmind.com";
const METADATA_ENDPOINT: &str = "/geoip/updates/metadata?edition_id=";
const DOWNLOAD_ENDPOINT: &str = "/geoip/databases/";

#[derive(Default)]
pub struct LookupResult {
    pub city: Option<String>,
    pub country_code: Option<String>,
}

pub struct LiwanGeoIP {
    reader: ArcSwapOption<maxminddb::Reader<Vec<u8>>>,
    update_lock: Mutex<()>,
    geoip: crate::config::GeoIpConfig,
    path: PathBuf,
}

impl LiwanGeoIP {
    pub fn try_new(config: crate::config::Config) -> Result<Self> {
        let geoip = config.geoip;
        if geoip.maxmind_account_id.is_none() && geoip.maxmind_license_key.is_none() && geoip.maxmind_db_path.is_none()
        {
            tracing::trace!("GeoIP support disabled, skipping...");
            return Ok(Self::noop());
        }

        let edition = &geoip.maxmind_edition;
        let default_path = PathBuf::from(config.data_dir.clone()).join(format!("./geoip/{edition}.mmdb"));
        let path = geoip.maxmind_db_path.as_ref().map_or(default_path, PathBuf::from);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if path.extension() != Some("mmdb".as_ref()) {
            return Err(anyhow!("Invalid GeoIP database path file extension, expected '.mmdb'"));
        }

        tracing::info!(database = geoip.maxmind_db_path, "GeoIP support enabled, loading database");

        let reader = path
            .exists()
            .then(|| maxminddb::Reader::open_readfile(&path).map(Arc::new))
            .transpose()
            .with_context(|| format!("Failed to open GeoIP database at {}", path.display()))?;

        Ok(Self { geoip, reader: ArcSwapOption::new(reader), path, update_lock: Default::default() })
    }

    fn noop() -> Self {
        Self {
            geoip: Default::default(),
            reader: ArcSwapOption::new(None),
            update_lock: Default::default(),
            path: PathBuf::new(),
        }
    }

    /// Lookup an IP address in the loaded GeoIP database
    pub fn lookup(&self, ip: &IpAddr) -> Result<LookupResult> {
        let Some(reader) = &*self.reader.load() else {
            return Ok(Default::default());
        };

        let lookup = reader.lookup(*ip)?.decode::<maxminddb::geoip2::City>().context("failed to decode data")?;
        let city = lookup.as_ref().and_then(|lookup| lookup.city.names.english.map(|v| v.to_string()));
        let country_code = lookup.and_then(|lookup| lookup.country.iso_code.map(|v| v.to_string()));
        Ok(LookupResult { city, country_code })
    }

    /// Check for updates and download the latest database if available
    pub async fn check_for_updates(&self) -> Result<()> {
        let Ok(_update) = self.update_lock.try_lock() else {
            return Ok(());
        };

        let maxmind_edition = &self.geoip.maxmind_edition;
        let maxmind_account_id =
            self.geoip.maxmind_account_id.as_ref().ok_or_else(|| anyhow!("MaxMind account ID not found"))?.to_string();
        let maxmind_license_key =
            self.geoip.maxmind_license_key.as_deref().ok_or_else(|| anyhow!("MaxMind license key not found"))?;

        let db_exists = self.path.exists();
        let db_md5 = if db_exists {
            let path = self.path.clone();
            tokio::task::spawn_blocking(move || file_md5(&path)).await??
        } else {
            String::new()
        };

        let mut update = !db_exists;
        if db_exists {
            match get_latest_md5(maxmind_edition, &maxmind_account_id, maxmind_license_key).await {
                Ok(latest_md5) => {
                    if latest_md5 != db_md5 {
                        tracing::info!("GeoIP database outdated, downloading...");
                        update = true;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "Failed to get latest MaxMind database MD5 hash, skipping update");
                }
            };
        } else {
            tracing::info!("GeoIP database doesn't exist, attempting to download...");
        }

        if update {
            let file = match download_maxmind_db(maxmind_edition, &maxmind_account_id, maxmind_license_key).await {
                Ok(file) => file,
                Err(e) => {
                    tracing::warn!(error = ?e, "Failed to download GeoIP database, skipping update");
                    return Ok(());
                }
            };

            // The current reader keeps serving lookups while the file is copied.
            let copied = tokio::fs::copy(&file, &self.path).await;
            let removed = tokio::fs::remove_file(file).await;
            copied?;
            removed?;

            let path = std::fs::canonicalize(&self.path)?;
            tracing::info!(path = ?path, "GeoIP database updated successfully");
        }

        Ok(())
    }

    /// Reload the database from disk (as long as no update is currently in progress)
    pub fn reload(&self) -> Result<()> {
        let Ok(_update) = self.update_lock.try_lock() else {
            return Ok(());
        };

        let reader = maxminddb::Reader::open_readfile(self.path.clone())?;
        self.reader.store(Some(reader.into()));
        Ok(())
    }
}

/// Keep the GeoIP database refreshed and reload it after local file changes
pub async fn keep_updated(geoip: Arc<LiwanGeoIP>) {
    if geoip.path.as_os_str().is_empty() {
        return;
    }
    let path: PathBuf = geoip.path.clone();
    let mut last_meta = get_file_meta(&path);

    let mut file_interval = tokio::time::interval(Duration::from_secs(60));
    let mut daily_interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
    let enable_updates = geoip.geoip.maxmind_account_id.is_some() && geoip.geoip.maxmind_license_key.is_some();

    loop {
        tokio::select! {
            _ = file_interval.tick() => {
                // just a simple polling based file watcher so we don't need to add a bunch of dependencies
                let meta = get_file_meta(&path);
                if meta != last_meta {
                    if let Err(e) = geoip.reload() {
                        tracing::error!(error = ?e, "Failed to reload GeoIP database");
                    } else {
                        tracing::info!("GeoIP database reloaded after file change");
                        last_meta = meta;
                    }
                }
            }
            _ = daily_interval.tick()  => {
                if enable_updates && let Err(e) = geoip.check_for_updates().await {
                    tracing::error!(error = ?e, "Failed to check for GeoIP database updates");
                }
            }
        }
    }
}

fn get_file_meta(path: &PathBuf) -> Option<(u64, u64, u64, i64)> {
    let md = std::fs::metadata(path).ok()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some((md.dev(), md.ino(), md.size(), md.mtime()))
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return Some((md.file_index(), md.volume_serial_number(), md.file_size(), md.last_write_time() as i64));
    };

    #[cfg(not(any(unix, windows)))]
    Some((0, 0, md.len(), md.modified().ok()?.elapsed().ok()?.as_secs() as i64))
}

async fn get_latest_md5(edition: &str, account_id: &str, license_key: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct Metadata {
        databases: Vec<Database>,
    }

    #[derive(serde::Deserialize)]
    struct Database {
        md5: String,
    }

    let url = format!("{BASE_URL}{METADATA_ENDPOINT}{edition}");
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .basic_auth(account_id, Some(license_key))
        .send()
        .await?
        .error_for_status()?
        .json::<Metadata>()
        .await?;

    response.databases.into_iter().next().map(|database| database.md5).ok_or_else(|| anyhow!("No databases found"))
}

fn file_md5(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = md5::Md5::new();

    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(crate::utils::encode_hex(&hasher.finalize()))
}

async fn download_maxmind_db(edition: &str, account_id: &str, license_key: &str) -> Result<PathBuf> {
    let url = format!("{BASE_URL}{DOWNLOAD_ENDPOINT}{edition}/download?suffix=tar.gz");

    let client = reqwest::Client::new();
    let response = client.get(url).basic_auth(account_id, Some(license_key)).send().await?.error_for_status()?;
    let stream = response.bytes_stream().map(|b| b.map_err(io::Error::other));
    let stream = StreamReader::new(stream);
    let stream = async_compression::tokio::bufread::GzipDecoder::new(stream);
    extract_maxmind_db(stream).await
}

async fn extract_maxmind_db(reader: impl AsyncRead + Unpin) -> Result<PathBuf> {
    let mut archive = TarArchive::new(reader);

    while let Some(member) = archive.next_member().await.context("Failed to read GeoIP archive entry")? {
        let Member::File { metadata, mut payload, .. } = member else {
            continue;
        };

        if Path::new(&metadata.path).extension().is_some_and(|ext| ext == "mmdb") {
            let suffix = Alphanumeric.sample_string(&mut rand::rng(), 32);
            let path = std::env::temp_dir().join(format!("liwan-geoip-{suffix}.mmdb"));
            let mut file =
                tokio::fs::File::create_new(&path).await.context("Failed to create temporary GeoIP database file")?;
            let result: Result<()> = async {
                let mut buffer = Vec::new();
                while payload
                    .next_chunk(&mut buffer, 64 * 1024)
                    .await
                    .context("Failed to read GeoIP database payload")?
                {
                    file.write_all(&buffer).await?;
                }
                file.flush().await?;
                Ok(())
            }
            .await;
            drop(file);
            if let Err(error) = result {
                let _ = tokio::fs::remove_file(&path).await;
                return Err(error);
            }
            return Ok(path);
        }
    }

    Err(anyhow!("No .mmdb file found in GeoIP archive"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tar_codec::{ArchiveBuilder, EntryMetadata, TarEncoder};

    #[tokio::test]
    async fn extracts_only_the_database_file() {
        let mut archive = TarEncoder::new(Vec::new()).builder();
        archive.add_directory("ignored.mmdb").await.unwrap();
        archive.add_file("README.txt", b"readme".as_slice(), EntryMetadata::default()).await.unwrap();
        let database = vec![42; 128 * 1024 + 1];
        archive.add_file("GeoLite2/GeoLite2-City.mmdb", database.as_slice(), EntryMetadata::default()).await.unwrap();
        let archive = archive.finish_into_inner().await.unwrap().into_inner();

        let path = extract_maxmind_db(archive.as_slice()).await.unwrap();
        let extracted = std::fs::read(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(extracted, database);
    }

    #[tokio::test]
    async fn rejects_archive_without_database_file() {
        let mut archive = TarEncoder::new(Vec::new()).builder();
        archive.add_directory("ignored.mmdb").await.unwrap();
        let archive = archive.finish_into_inner().await.unwrap().into_inner();

        assert!(extract_maxmind_db(archive.as_slice()).await.is_err());
    }
}
