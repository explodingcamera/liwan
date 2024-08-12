use crate::config::Config;
use crate::utils::refinery_duckdb::DuckDBConnection;
use crate::utils::refinery_sqlite::RqlConnection;

use core::{LiwanEntities, LiwanEvents, LiwanOnboarding, LiwanProjects, LiwanSessions, LiwanUsers};
use duckdb::DuckdbConnectionManager;
use eyre::Result;
use r2d2_sqlite::SqliteConnectionManager;
use refinery::Runner;
use std::path::PathBuf;
use std::sync::Arc;

mod core;
pub(crate) mod models;
pub(crate) mod reports;

pub(crate) type DuckDBConn = r2d2::PooledConnection<DuckdbConnectionManager>;
pub(crate) type DuckDBPool = r2d2::Pool<DuckdbConnectionManager>;
pub(crate) type SqlitePool = r2d2::Pool<SqliteConnectionManager>;

#[derive(Clone)]
pub(crate) struct Liwan {
    events_pool: r2d2::Pool<DuckdbConnectionManager>,

    pub(crate) events: core::events::LiwanEvents,
    pub(crate) users: core::users::LiwanUsers,
    pub(crate) sessions: core::sessions::LiwanSessions,
    pub(crate) onboarding: core::onboarding::LiwanOnboarding,
    pub(crate) entities: core::entities::LiwanEntities,
    pub(crate) projects: core::projects::LiwanProjects,

    pub(crate) config: Arc<Config>,
}

#[rustfmt::skip]
mod embedded {
    pub(super) mod app { refinery::embed_migrations!("src/migrations/app"); }
    pub(super) mod events { refinery::embed_migrations!("src/migrations/events"); }
}

const EVENT_BATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

impl Liwan {
    pub(crate) fn try_new(config: Config) -> Result<Self> {
        tracing::debug!("Initializing app");
        let folder = std::path::Path::new(&config.db_dir);
        if !folder.exists() {
            tracing::debug!(path = config.db_dir, "Creating database folder since it doesn't exist");
            std::fs::create_dir_all(folder)?;
        }

        tracing::debug!("Initializing databases");
        let conn_app = init_sqlite(&folder.join("liwan-app.sqlite"), embedded::app::migrations::runner())?;
        let conn_events = init_duckdb(&folder.join("liwan-events.duckdb"), embedded::events::migrations::runner())?;

        Ok(Self {
            events: LiwanEvents::try_new(conn_events.clone(), conn_app.clone())?,
            onboarding: LiwanOnboarding::try_new(conn_app.clone())?,
            sessions: LiwanSessions::new(conn_app.clone()),
            entities: LiwanEntities::new(conn_app.clone()),
            projects: LiwanProjects::new(conn_app.clone()),
            users: LiwanUsers::new(conn_app),

            events_pool: conn_events,
            config: config.into(),
        })
    }

    pub(crate) fn events_conn(&self) -> Result<DuckDBConn> {
        Ok(self.events_pool.get()?)
    }
}

fn init_duckdb(path: &PathBuf, mut migrations_runner: Runner) -> Result<r2d2::Pool<DuckdbConnectionManager>> {
    let conn = DuckdbConnectionManager::file(path)?;
    let pool = r2d2::Pool::new(conn)?;
    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run(&mut DuckDBConnection(pool.get()?))?;
    pool.get()?.execute_batch("set enable_fsst_vectors = true")?;
    Ok(pool)
}

fn init_sqlite(path: &PathBuf, mut migrations_runner: Runner) -> Result<r2d2::Pool<SqliteConnectionManager>> {
    let conn = SqliteConnectionManager::file(path);
    let pool = r2d2::Pool::new(conn)?;
    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run(&mut RqlConnection(pool.get()?))?;
    Ok(pool)
}
