use crate::config::DuckdbConfig;
use crate::utils::refinery_duckdb::DuckDBConnection;
use crate::utils::refinery_sqlite::RqlConnection;

use duckdb::DuckdbConnectionManager;
use eyre::{Result, bail};
use r2d2_sqlite::SqliteConnectionManager;
use refinery::Runner;
use std::path::PathBuf;

pub(super) fn init_duckdb(
    path: &PathBuf,
    duckdb_config: Option<DuckdbConfig>,
    mut migrations_runner: Runner,
) -> Result<r2d2::Pool<DuckdbConnectionManager>> {
    let conn = DuckdbConnectionManager::file(path)?;
    let pool = r2d2::Pool::new(conn)?;
    migrations_runner.set_migration_table_name("migrations");

    for migration in migrations_runner.run_iter(&mut DuckDBConnection(pool.get()?)) {
        match migration {
            Ok(migration) => {
                tracing::info!("Applied migration: {}", migration);
            }
            Err(err) => {
                bail!("Failed to apply migration: {}", err);
            }
        }
    }

    {
        let conn = pool.get()?;
        conn.pragma_update(None, "allow_community_extensions", &"false")?;
        conn.pragma_update(None, "enable_external_access", &"false")?;
        conn.pragma_update(None, "enable_fsst_vectors", &"true")?;

        if let Some(duckdb_config) = duckdb_config {
            if let Some(memory_limit) = duckdb_config.memory_limit {
                conn.pragma_update(None, "memory_limit", &memory_limit)?;
            }

            if let Some(threads) = duckdb_config.threads {
                conn.pragma_update(None, "threads", &threads.to_string())?;
            }
        }
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
        conn.pragma_update(None, "autoinstall_known_extensions", &"false")?;
        conn.pragma_update(None, "autoload_known_extensions", &"false")?;
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
        conn.pragma_update(None, "foreign_keys", "ON")?;
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

    {
        let conn = pool.get()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
    }

    Ok(pool)
}
