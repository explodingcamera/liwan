use crate::config::DuckdbConfig;
use crate::utils::duckdb::{LATEST_STORAGE_FORMAT, requires_newer_duckdb, storage_format};
use crate::utils::refinery_duckdb::DuckDBConnection;
use crate::utils::refinery_sqlite::RqlConnection;

use crate::utils::r2d2_sqlite::SqliteConnectionManager;
use anyhow::{Context, Result, bail};
use duckdb::DuckdbConnectionManager;
use refinery::Runner;
use std::path::PathBuf;

pub(super) fn init_duckdb(
    path: &PathBuf,
    duckdb_config: DuckdbConfig,
    mut migrations_runner: Runner,
) -> Result<r2d2::Pool<DuckdbConnectionManager>> {
    let mut tries = 10;
    let conn = loop {
        let mut flags = duckdb::Config::default()
            .enable_autoload_extension(true)?
            .access_mode(duckdb::AccessMode::ReadWrite)?
            .with("enable_fsst_vectors", "true")?
            .with("allocator_background_threads", "true")?;

        if let Some(memory_limit) = &duckdb_config.memory_limit {
            flags = flags.max_memory(memory_limit)?;
        }

        if let Some(threads) = duckdb_config.threads {
            flags = flags.threads(threads.get().into())?;
        }

        for (name, value) in [
            ("temp_directory", &duckdb_config.temp_directory),
            ("max_temp_directory_size", &duckdb_config.max_temp_directory_size),
            ("checkpoint_threshold", &duckdb_config.checkpoint_threshold),
        ] {
            if let Some(value) = value {
                flags = flags.with(name, value).with_context(|| format!("Invalid duckdb.{name}"))?;
            }
        }

        match DuckdbConnectionManager::file_with_flags(path, flags) {
            Ok(conn) => break conn,
            Err(e) => {
                let message = e.to_string();
                if requires_newer_duckdb(&message) {
                    return Err(e).with_context(|| {
                        format!("{} uses a newer DuckDB storage format than this Liwan build supports", path.display())
                    });
                }
                if tries <= 0 {
                    tracing::warn!("");
                    return Err(e).context("Failed to load DuckDB Database after 10 attempts");
                }

                if message.contains("Could not set lock on file") {
                    tracing::warn!("DuckDB database is locked. Retrying... ({} tries left)", tries);
                    tries -= 1;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                } else {
                    return Err(e).context("Failed to load DuckDB Database");
                }
            }
        }
    };

    let pool = r2d2::Pool::new(conn)?;
    {
        let conn = pool.get()?;
        conn.execute("PRAGMA enable_checkpoint_on_shutdown", [])?;
        conn.pragma_update(None, "autoload_known_extensions", &"true")?;
        conn.pragma_update(None, "allow_community_extensions", &"false")?;

        match storage_format(&conn, None) {
            Ok(current) if current != LATEST_STORAGE_FORMAT => tracing::info!(
                path = %path.display(),
                current_format = %current,
                latest_format = LATEST_STORAGE_FORMAT,
                "A newer DuckDB storage format is available. Stop Liwan and run `liwan upgrade-duckdb` to upgrade the file."
            ),
            Err(err) => tracing::debug!(%err, "Could not read DuckDB storage format"),
            _ => {}
        }

        conn.execute_batch("CHECKPOINT").context("Failed to checkpoint DuckDB before migrations")?;
        migrations_runner.set_migration_table_name("migrations");
        let mut conn = DuckDBConnection(conn);
        for migration in migrations_runner.run_iter(&mut conn) {
            match migration {
                Ok(migration) => {
                    tracing::info!("Applied migration: {}", migration);
                }
                Err(err) => {
                    bail!("Failed to apply migration: {}", err);
                }
            }
        }
        conn.0.execute_batch("CHECKPOINT").context("Failed to checkpoint DuckDB after migrations")?;
    }

    Ok(pool)
}

pub fn init_duckdb_mem(mut migrations_runner: Runner) -> Result<r2d2::Pool<DuckdbConnectionManager>> {
    let conn = DuckdbConnectionManager::memory()?;
    let pool = r2d2::Pool::new(conn)?;
    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run(&mut DuckDBConnection(pool.get()?))?;

    {
        let conn = pool.get()?;
        conn.pragma_update(None, "allow_community_extensions", &"false")?;
        conn.pragma_update(None, "enable_fsst_vectors", &"true")?;
    }

    Ok(pool)
}

pub(super) fn init_sqlite(
    path: &PathBuf,
    mut migrations_runner: Runner,
) -> Result<r2d2::Pool<SqliteConnectionManager>> {
    let conn = SqliteConnectionManager::file(path);
    let pool = r2d2::Pool::new(conn)?;
    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run(&mut RqlConnection(pool.get()?))?;

    {
        let conn = pool.get()?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "mmap_size", "268435456")?;
        conn.pragma_update(None, "journal_size_limit", "268435456")?;
        conn.pragma_update(None, "cache_size", "2000")?;
    }

    Ok(pool)
}

pub fn init_sqlite_mem(mut migrations_runner: Runner) -> Result<r2d2::Pool<SqliteConnectionManager>> {
    let conn = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::new(conn)?;
    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run(&mut RqlConnection(pool.get()?))?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::embedded::events::migrations;

    #[test]
    fn checkpoints_event_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        let wal = path.with_extension("duckdb.wal");
        let pool = init_duckdb(&path, DuckdbConfig::default(), migrations::runner()).unwrap();
        let conn = pool.get().unwrap();

        conn.execute_batch("SELECT properties FROM events").unwrap();
        assert!(std::fs::metadata(wal).map_or_else(|err| err.kind() == std::io::ErrorKind::NotFound, |m| m.len() == 0));
    }

    #[test]
    fn applies_checkpoint_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        let config = DuckdbConfig { checkpoint_threshold: Some("64MiB".to_string()), ..Default::default() };
        let pool = init_duckdb(&path, config, migrations::runner()).unwrap();
        let threshold: String = pool
            .get()
            .unwrap()
            .query_row("SELECT current_setting('checkpoint_threshold')", [], |row| row.get(0))
            .unwrap();
        assert_eq!(threshold, "64.0 MiB");
    }

    #[test]
    fn applies_temp_storage_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        let spill_dir = dir.path().join("spill").to_str().unwrap().to_string();
        let config = DuckdbConfig {
            temp_directory: Some(spill_dir.clone()),
            max_temp_directory_size: Some("2GiB".to_string()),
            ..Default::default()
        };
        let pool = init_duckdb(&path, config, migrations::runner()).unwrap();
        let (actual_dir, actual_size): (String, String) = pool
            .get()
            .unwrap()
            .query_row(
                "SELECT current_setting('temp_directory'), current_setting('max_temp_directory_size')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(actual_dir, spill_dir);
        assert_eq!(actual_size, "2.0 GiB");
    }
}
