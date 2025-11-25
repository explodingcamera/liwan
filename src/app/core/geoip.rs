#![allow(dead_code)]

use std::io::{self};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::app::SqlitePool;
use arc_swap::ArcSwapOption;
use eyre::{Context, OptionExt, Result};
use futures_lite::StreamExt;
use md5::{Digest, Md5};
use tokio_tar::Archive;
use tokio_util::io::StreamReader;

const BASE_URL: &str = "https://updates.maxmind.com";
const METADATA_ENDPOINT: &str = "/geoip/updates/metadata?edition_id=";
const DOWNLOAD_ENDPOINT: &str = "/geoip/databases/";

#[derive(Default)]
pub struct LookupResult {
    pub city: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Clone)]
pub struct LiwanGeoIP {
    pool: SqlitePool,
    reader: Arc<ArcSwapOption<maxminddb::Reader<Vec<u8>>>>,

    downloading: Arc<AtomicBool>,
    geoip: crate::config::GeoIpConfig,
    path: PathBuf,
}

impl LiwanGeoIP {
    pub fn try_new(config: crate::config::Config, pool: SqlitePool) -> Result<Self> {
        let geoip = config.geoip;
        if geoip.maxmind_account_id.is_none() && geoip.maxmind_license_key.is_none() && geoip.maxmind_db_path.is_none()
        {
            tracing::trace!("GeoIP support disabled, skipping...");
            return Ok(Self::noop(pool));
        }

        let edition = &geoip.maxmind_edition;
        let default_path = PathBuf::from(config.data_dir.clone()).join(format!("./geoip/{edition}.mmdb"));
        let path = geoip.maxmind_db_path.as_ref().map_or(default_path, PathBuf::from);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if path.extension() != Some("mmdb".as_ref()) {
            return Err(eyre::eyre!("Invalid GeoIP database path file extension, expected '.mmdb'"));
        }

        tracing::info!(database = geoip.maxmind_db_path, "GeoIP support enabled, loading database");

        let reader = path.exists().then(|| {
            maxminddb::Reader::open_readfile(path.clone()).expect("Failed to open GeoIP database file").into()
        });

        Ok(Self { geoip, pool, reader: ArcSwapOption::new(reader).into(), path, downloading: Default::default() })
    }

    fn is_enabled(&self) -> bool {
        self.reader.load().is_some() || self.downloading.load(Ordering::Acquire)
    }

    fn noop(pool: SqlitePool) -> Self {
        Self {
            geoip: Default::default(),
            pool,
            reader: ArcSwapOption::new(None).into(),
            downloading: Default::default(),
            path: PathBuf::new(),
        }
    }

    // Lookup the IP address in the GeoIP database
    pub fn lookup(&self, ip: &IpAddr) -> Result<LookupResult> {
        let Some(reader) = &*self.reader.load() else {
            return Ok(Default::default());
        };

        let lookup = reader
            .lookup::<maxminddb::geoip2::City>(*ip)?
            .ok_or_else(|| eyre::eyre!("No data found for IP address"))?;

        let city = lookup.city.and_then(|city| city.names.and_then(|names| names.get("en").map(|s| (*s).to_string())));
        let country_code = lookup.country.and_then(|country| country.iso_code.map(ToString::to_string));
        Ok(LookupResult { city, country_code })
    }

    // Check for updates and download the latest database if available
    pub async fn check_for_updates(&self) -> Result<()> {
        if self.downloading.swap(true, Ordering::Acquire) {
            return Ok(());
        }

        let maxmind_edition = &self.geoip.maxmind_edition;
        let maxmind_account_id = self.geoip.maxmind_account_id.as_deref().ok_or_eyre("MaxMind account ID not found")?;
        let maxmind_license_key =
            self.geoip.maxmind_license_key.as_deref().ok_or_eyre("MaxMind license key not found")?;

        let db_exists = self.path.exists();
        let db_md5 = if db_exists { file_md5(&self.path)? } else { String::new() };

        let mut update = !db_exists;
        if db_exists {
            match get_latest_md5(maxmind_edition, maxmind_account_id, maxmind_license_key).await {
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
            let file = match download_maxmind_db(maxmind_edition, maxmind_account_id, maxmind_license_key).await {
                Ok(file) => file,
                Err(e) => {
                    tracing::warn!(error = ?e, "Failed to download GeoIP database, skipping update");
                    self.downloading.store(false, Ordering::Release);
                    return Ok(());
                }
            };

            // close the current reader to free up the file
            self.reader.swap(None);

            // move the downloaded file to the correct path
            std::fs::copy(&file, &self.path)?;
            std::fs::remove_file(file)?;

            // open the new reader
            let reader = maxminddb::Reader::open_readfile(self.path.clone())?;
            self.reader.store(Some(reader.into()));

            let path = std::fs::canonicalize(&self.path)?;
            tracing::info!(path = ?path, "GeoIP database updated successfully");
        }

        self.downloading.store(false, Ordering::Release);
        Ok(())
    }
}

pub fn keep_updated(geoip: LiwanGeoIP) {
    if !geoip.is_enabled() {
        return;
    }

    tokio::task::spawn(async move {
        if let Err(e) = geoip.check_for_updates().await {
            tracing::error!(error = ?e, "Failed to check for GeoIP database updates");
        }

        // Create an interval that ticks every 24 hours
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(86400));
        loop {
            interval.tick().await;
            let geoip = geoip.clone();
            // Run the task once a day
            tokio::task::spawn(async move {
                if let Err(e) = geoip.check_for_updates().await {
                    tracing::error!(error = ?e, "Failed to check for GeoIP database updates");
                }
            });
        }
    });
}

async fn get_latest_md5(edition: &str, account_id: &str, license_key: &str) -> Result<String> {
    let url = format!("{BASE_URL}{METADATA_ENDPOINT}{edition}");
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .basic_auth(account_id, Some(license_key))
        .send()
        .await?
        .json::<ahash::HashMap<String, Vec<ahash::HashMap<String, String>>>>()
        .await?;

    Ok(response
        .get("databases")
        .ok_or_eyre("No databases found")?
        .first()
        .ok_or_eyre("MD5 hash not found")?
        .get("md5")
        .ok_or_eyre("MD5 hash not found")?
        .clone())
}

fn file_md5(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Md5::new();
    std::io::copy(&mut reader, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

async fn download_maxmind_db(edition: &str, account_id: &str, license_key: &str) -> Result<PathBuf> {
    let url = format!("{BASE_URL}{DOWNLOAD_ENDPOINT}{edition}/download?suffix=tar.gz");

    let client = reqwest::Client::new();
    let response = client.get(url).basic_auth(account_id, Some(license_key)).send().await?.error_for_status()?;
    let stream = response.bytes_stream().map(|b| b.map_err(io::Error::other));
    let stream = StreamReader::new(stream);
    let stream = async_compression::tokio::bufread::GzipDecoder::new(stream);
    let mut archive = Archive::new(stream);
    let mut entries = archive.entries()?;

    let folder = std::env::temp_dir().join("liwan-geoip");
    std::fs::create_dir_all(&folder).wrap_err("Failed to create temp directory")?;

    let file;
    loop {
        let mut entry = entries.next().await.ok_or_eyre("No entries found")?.wrap_err("Failed to read entry")?;

        let entry_path = entry.path()?;
        if entry_path.extension().is_some_and(|ext| ext == "mmdb") {
            entry.set_allow_external_symlinks(false);
            entry.set_preserve_permissions(false);

            file = entry
                .unpack_in(folder)
                .await
                .wrap_err("Failed to unpack entry")?
                .ok_or_eyre("Failed to unpack entry")?;

            break;
        }
    }

    Ok(file)
}
