use crate::config::DuckdbConfig;
use crate::utils::hash::db_name;
use crate::utils::refinery_duckdb::DuckDBConnection;
use crate::utils::refinery_sqlite::SqlxConnection;

use anyhow::{Context, Result, bail};
use duckdb::DuckdbConnectionManager;
use refinery::Runner;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{SqlitePool, sqlite::SqliteJournalMode, sqlite::SqliteSynchronous};
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

        match DuckdbConnectionManager::file_with_flags(path, flags) {
            Ok(conn) => break conn,
            Err(e) => {
                if tries <= 0 {
                    tracing::warn!("");
                    return Err(e).context("Failed to load DuckDB Database after 10 attempts");
                }

                if e.to_string().contains("Could not set lock on file") {
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

/// Connection options for the app database, either from a `DATABASE` url or from the data directory
pub(super) fn database_options(database: Option<&str>, path: &PathBuf) -> Result<SqliteConnectOptions> {
    let options = match database {
        Some(database) => {
            database.parse::<SqliteConnectOptions>().context("Failed to parse the database connection url")?
        }
        None => SqliteConnectOptions::new().filename(path).create_if_missing(true),
    };

    Ok(options
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .pragma("mmap_size", "268435456")
        .pragma("journal_size_limit", "268435456")
        .pragma("cache_size", "2000"))
}

pub(super) async fn init_database(options: SqliteConnectOptions, mut migrations_runner: Runner) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run_async(&mut SqlxConnection(&pool)).await?;
    Ok(pool)
}

pub async fn init_database_mem(mut migrations_runner: Runner) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(format!("file:{}", db_name()))
        .in_memory(true)
        .shared_cache(true)
        .foreign_keys(true);

    // in-memory databases are dropped as soon as the last connection to them is closed
    let pool =
        SqlitePoolOptions::new().min_connections(1).idle_timeout(None).max_lifetime(None).connect_with(options).await?;

    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run_async(&mut SqlxConnection(&pool)).await?;
    Ok(pool)
}
