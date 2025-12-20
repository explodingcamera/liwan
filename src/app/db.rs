use crate::config::DuckdbConfig;
use crate::utils::refinery_duckdb::DuckDBConnection;
use crate::utils::refinery_sqlite::RqlConnection;

use crate::utils::r2d2_sqlite::SqliteConnectionManager;
use anyhow::{Result, bail};
use duckdb::DuckdbConnectionManager;
use refinery::Runner;
use std::path::PathBuf;

pub(super) fn init_duckdb(
    path: &PathBuf,
    duckdb_config: DuckdbConfig,
    mut migrations_runner: Runner,
) -> Result<r2d2::Pool<DuckdbConnectionManager>> {
    let mut flags = duckdb::Config::default()
        .enable_autoload_extension(true)?
        .access_mode(duckdb::AccessMode::ReadWrite)?
        .with("enable_fsst_vectors", "true")?
        .with("allocator_background_threads", "true")?;

    if let Some(memory_limit) = duckdb_config.memory_limit {
        flags = flags.max_memory(&memory_limit)?;
    }

    if let Some(threads) = duckdb_config.threads {
        flags = flags.threads(threads.get().into())?;
    }

    let conn = DuckdbConnectionManager::file_with_flags(path, flags).map_err(|e| {
        tracing::warn!("Failed to create DuckDB connection. If you've just upgraded to Liwan 1.2, please downgrade to version 1.1.1 first, start and stop the server, and then upgrade to 1.2 again.");
        anyhow::anyhow!("Failed to create DuckDB connection: {}", e)
    })?;

    let pool = r2d2::Pool::new(conn)?;
    {
        let conn = pool.get()?;
        conn.execute("PRAGMA enable_checkpoint_on_shutdown", [])?;
        conn.pragma_update(None, "autoload_known_extensions", &"true")?;
        conn.pragma_update(None, "allow_community_extensions", &"false")?;
    }

    {
        let conn = pool.get()?;
        migrations_runner.set_migration_table_name("migrations");
        for migration in migrations_runner.run_iter(&mut DuckDBConnection(conn)) {
            match migration {
                Ok(migration) => {
                    tracing::info!("Applied migration: {}", migration);
                }
                Err(err) => {
                    bail!("Failed to apply migration: {}", err);
                }
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
