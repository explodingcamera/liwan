#![allow(dead_code)]

use std::{
    collections::HashMap,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::app::SqlitePool;
use crossbeam::sync::ShardedLock;
use eyre::{OptionExt, Result};
use futures_util::{StreamExt, TryStreamExt};
use md5::{Digest, Md5};
use tokio_tar::Archive;
use tokio_util::io::StreamReader;

const BASE_URL: &str = "https://updates.maxmind.com";
const METADATA_ENDPOINT: &str = "/geoip/updates/metadata?edition_id=";
const DOWNLOAD_ENDPOINT: &str = "/geoip/databases/";

pub struct LookupResult {
    pub city: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Clone)]
pub struct LiwanGeoIP {
    pool: SqlitePool,
    reader: Arc<ShardedLock<Option<maxminddb::Reader<Vec<u8>>>>>,

    downloading: Arc<AtomicBool>,
    config: crate::config::Config,
    geoip: crate::config::GeoIpConfig,
    path: PathBuf,
}

impl LiwanGeoIP {
    pub fn try_new(config: crate::config::Config, pool: SqlitePool) -> Result<Option<Self>> {
        let Some(geoip) = &config.geoip else {
            tracing::trace!("GeoIP support disabled, skipping...");
            return Ok(None);
        };

        if geoip.maxmind_account_id.is_none() && geoip.maxmind_license_key.is_none() && geoip.maxmind_db_path.is_none()
        {
            tracing::trace!("GeoIP support disabled, skipping...");
            return Ok(None);
        }

        let edition = geoip.maxmind_edition.as_deref().unwrap_or("GeoLite2-City");
        let default_path = PathBuf::from(config.data_dir.clone()).join(format!("./geoip/{}.mmdb", edition));
        let path = geoip.maxmind_db_path.as_ref().map(PathBuf::from).unwrap_or(default_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if path.extension() != Some("mmdb".as_ref()) {
            return Err(eyre::eyre!("Invalid GeoIP database path file extension, expected '.mmdb'"));
        }

        tracing::info!(database = geoip.maxmind_db_path, "GeoIP support enabled, loading database");
        let reader = if path.exists() {
            Some(maxminddb::Reader::open_readfile(path.clone()).expect("Failed to open GeoIP database file"))
        } else {
            None
        };

        Ok(Some(Self {
            geoip: geoip.clone(),
            config,
            pool,
            reader: Arc::new(ShardedLock::new(reader)),
            path,
            downloading: Arc::new(AtomicBool::new(false)),
        }))
    }

    // Lookup the IP address in the GeoIP database
    pub fn lookup(&self, ip: &IpAddr) -> Result<LookupResult> {
        let reader = self.reader.read().map_err(|_| eyre::eyre!("Failed to acquire GeoIP reader lock"))?;
        let reader = reader.as_ref().ok_or_eyre("GeoIP database not found")?;
        let lookup: maxminddb::geoip2::City = reader.lookup(*ip)?;
        let city = lookup.city.and_then(|city| city.names.and_then(|names| names.get("en").map(|s| s.to_string())));
        let country_code = lookup.country.and_then(|country| country.iso_code.map(|s| s.to_string()));
        Ok(LookupResult { city, country_code })
    }

    // Check for updates and download the latest database if available
    pub async fn check_for_updates(&self) -> Result<()> {
        if self.downloading.swap(true, Ordering::Acquire) {
            return Ok(());
        }

        let maxmind_edition = self.geoip.maxmind_edition.clone().ok_or_eyre("MaxMind edition not found")?;
        let maxmind_account_id = self.geoip.maxmind_account_id.clone().ok_or_eyre("MaxMind account ID not found")?;
        let maxmind_license_key = self.geoip.maxmind_license_key.clone().ok_or_eyre("MaxMind license key not found")?;

        let db_exists = self.path.exists();
        let db_md5 = if db_exists { file_md5(&self.path)? } else { String::new() };

        let mut update = false;
        if !db_exists {
            tracing::info!("GeoIP database doesn't exist, attempting to download...");
            update = true;
        } else {
            match get_latest_md5(&maxmind_edition, &maxmind_account_id, &maxmind_license_key).await {
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
        }

        if update {
            let Ok(file) = download_maxmind_db(&maxmind_edition, &maxmind_account_id, &maxmind_license_key).await
            else {
                tracing::warn!("Failed to download GeoIP database, skipping update");
                self.downloading.store(false, Ordering::Release);
                return Ok(());
            };

            // close the current reader to free up the file
            {
                let mut reader = self.reader.write().unwrap();
                reader.take();
            }

            // move the downloaded file to the correct path
            std::fs::copy(&file, &self.path)?;
            std::fs::remove_file(file)?;

            // open the new reader
            let reader = maxminddb::Reader::open_readfile(self.path.clone())?;
            *self.reader.write().unwrap() = Some(reader);

            tracing::info!(path = ?self.path, "GeoIP database updated successfully");
        }

        self.downloading.store(false, Ordering::Release);
        Ok(())
    }
}

pub fn keep_updated(geoip: Option<LiwanGeoIP>) {
    let Some(geoip) = geoip else { return };

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
    let url = format!("{}{}{}", BASE_URL, METADATA_ENDPOINT, edition);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .basic_auth(account_id, Some(license_key))
        .send()
        .await?
        .json::<HashMap<String, Vec<HashMap<String, String>>>>()
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
    let url = format!("{}{}{}/download?suffix=tar.gz", BASE_URL, DOWNLOAD_ENDPOINT, edition);

    let client = reqwest::Client::new();
    let response = client.get(url).basic_auth(account_id, Some(license_key)).send().await?.error_for_status()?;

    let stream = response.bytes_stream().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let stream = StreamReader::new(stream);
    let stream = async_compression::tokio::bufread::GzipDecoder::new(stream);
    let mut archive = Archive::new(stream);
    let mut entries = archive.entries()?;

    let folder = std::env::temp_dir().join("liwan-geoip");
    let file;
    loop {
        let mut entry = entries
            .next()
            .await
            .ok_or_else(|| eyre::eyre!("No entries found"))?
            .map_err(|e| eyre::eyre!("Failed to read entry: {}", e))?;

        let entry_path = entry.path()?;
        if entry_path.extension().map_or(false, |ext| ext == "mmdb") {
            file = entry
                .unpack_in(folder)
                .await
                .map_err(|e| eyre::eyre!("Failed to unpack entry: {}", e))?
                .ok_or_eyre("Failed to unpack entry")?;
            break;
        }
    }

    tracing::info!(file = ?file, "GeoIP database downloaded successfully");
    Ok(file)
}
