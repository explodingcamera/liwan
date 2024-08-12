use std::{path::PathBuf, sync::Arc};

use crossbeam::sync::ShardedLock;
use eyre::Result;

use crate::app::SqlitePool;

#[derive(Clone)]
pub(crate) struct LiwanGeoIP {
    pool: SqlitePool,
    reader: Arc<ShardedLock<maxminddb::Reader<Vec<u8>>>>,

    config: crate::config::Config,
    path: PathBuf,
}

impl LiwanGeoIP {
    pub(crate) fn try_new(config: crate::config::Config, pool: SqlitePool) -> Result<Option<Self>> {
        let Some(geoip) = &config.geoip else {
            tracing::trace!("GeoIP support disabled, skipping...");
            return Ok(None);
        };

        let edition = geoip.maxmind_edition.as_deref().unwrap_or("GeoLite2-City");
        let default_path = PathBuf::from(config.data_dir.clone()).join(format!("{}.mmdb", edition));
        let path = geoip.maxmind_db_path.as_ref().map(PathBuf::from).unwrap_or(default_path);

        tracing::info!(database = geoip.maxmind_db_path, "GeoIP support enabled, loading database");

        let reader = Arc::new(ShardedLock::new(
            maxminddb::Reader::open_readfile(path.clone()).expect("Failed to open GeoIP database file"),
        ));

        Ok(Some(Self { config, pool, reader, path }))
    }

    pub(crate) async fn check_for_updates(&self) -> Result<()> {
        Ok(())
    }
}
