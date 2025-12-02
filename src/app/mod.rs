mod core;
mod db;

pub mod models;
pub use core::reports;
use std::sync::Arc;

use crate::config::Config;

use anyhow::Result;
use core::{LiwanEntities, LiwanEvents, LiwanOnboarding, LiwanProjects, LiwanSessions, LiwanUsers};
use duckdb::DuckdbConnectionManager;
use r2d2_sqlite::SqliteConnectionManager;

pub type DuckDBConn = r2d2::PooledConnection<DuckdbConnectionManager>;
pub type DuckDBPool = r2d2::Pool<DuckdbConnectionManager>;
pub type SqlitePool = r2d2::Pool<SqliteConnectionManager>;

pub struct Liwan {
    events_pool: r2d2::Pool<DuckdbConnectionManager>,

    pub events: core::events::LiwanEvents,
    pub users: core::users::LiwanUsers,
    pub sessions: core::sessions::LiwanSessions,
    pub onboarding: core::onboarding::LiwanOnboarding,
    pub entities: core::entities::LiwanEntities,
    pub projects: core::projects::LiwanProjects,
    pub geoip: Arc<core::geoip::LiwanGeoIP>,

    pub config: Config,
}

#[rustfmt::skip]
mod embedded {
    pub(super) mod app { refinery::embed_migrations!("src/migrations/app"); }
    pub(super) mod events { refinery::embed_migrations!("src/migrations/events"); }
}

const EVENT_BATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

impl Liwan {
    pub fn try_new(config: Config) -> Result<Arc<Self>> {
        tracing::debug!("Initializing app");
        let folder = std::path::Path::new(&config.data_dir);
        if !folder.exists() {
            tracing::debug!(path = config.data_dir, "Creating database folder since it doesn't exist");
            std::fs::create_dir_all(folder)?;
        }

        tracing::debug!("Initializing databases");
        let conn_app = db::init_sqlite(&folder.join("liwan-app.sqlite"), embedded::app::migrations::runner())?;
        let conn_events = db::init_duckdb(
            &folder.join("liwan-events.duckdb"),
            config.duckdb.clone(),
            embedded::events::migrations::runner(),
        )?;

        Ok(Self {
            #[cfg(feature = "geoip")]
            geoip: core::geoip::LiwanGeoIP::try_new(config.clone(), conn_app.clone())?.into(),

            events: LiwanEvents::try_new(conn_events.clone(), conn_app.clone())?,
            onboarding: LiwanOnboarding::try_new(&conn_app)?,
            sessions: LiwanSessions::new(conn_app.clone()),
            entities: LiwanEntities::new(conn_app.clone()),
            projects: LiwanProjects::new(conn_app.clone()),
            users: LiwanUsers::new(conn_app),

            events_pool: conn_events,
            config,
        }
        .into())
    }

    pub fn new_memory(config: Config) -> Result<Arc<Self>> {
        tracing::debug!("Initializing app in memory");
        let conn_app = db::init_sqlite_mem(embedded::app::migrations::runner())?;
        let conn_events = db::init_duckdb_mem(embedded::events::migrations::runner())?;

        Ok(Self {
            #[cfg(feature = "geoip")]
            geoip: core::geoip::LiwanGeoIP::try_new(config.clone(), conn_app.clone())?.into(),

            events: LiwanEvents::try_new(conn_events.clone(), conn_app.clone())?,
            onboarding: LiwanOnboarding::try_new(&conn_app)?,
            sessions: LiwanSessions::new(conn_app.clone()),
            entities: LiwanEntities::new(conn_app.clone()),
            projects: LiwanProjects::new(conn_app.clone()),
            users: LiwanUsers::new(conn_app),

            events_pool: conn_events,
            config,
        }
        .into())
    }

    pub fn events_conn(&self) -> Result<DuckDBConn> {
        Ok(self.events_pool.get()?)
    }

    pub fn run_background_tasks(&self) {
        core::geoip::keep_updated(self.geoip.clone());
    }

    pub fn shutdown(&self) -> Result<()> {
        self.events_pool.get()?.execute("FORCE CHECKPOINT", [])?; // normal checkpoints don't seem to work consistently on shutdown
        tracing::info!("Shutting down");
        Ok(())
    }
}

#[cfg(any(debug_assertions, test, feature = "_enable_seeding"))]
impl Liwan {
    pub fn seed_database(&self, count_per_entity: usize) -> Result<()> {
        use chrono::{Days, Utc};
        use models::UserRole;

        let entities = vec![
            ("entity-1", "Entity 1", "example.com", vec!["public-project".to_string(), "private-project".to_string()]),
            // ("entity-2", "Entity 2", "test.example.com", vec!["private-project".to_string()]),
            // ("entity-3", "Entity 3", "example.org", vec!["public-project".to_string()]),
        ];
        let projects = [("public-project", "Public Project", true), ("private-project", "Private Project", false)];
        let users = [("admin", "admin", UserRole::Admin), ("user", "user", UserRole::User)];

        for (username, password, role) in users {
            self.users.create(username, password, role, &[])?;
        }

        for (project_id, display_name, public) in projects {
            self.projects.create(
                &models::Project {
                    id: project_id.to_string(),
                    display_name: display_name.to_string(),
                    public,
                    secret: None,
                },
                &[],
            )?;
        }

        let start = Utc::now().checked_sub_days(Days::new(365)).unwrap();
        let end = Utc::now();
        for (entity_id, display_name, fqdn, project_ids) in entities {
            self.entities.create(
                &models::Entity { id: entity_id.to_string(), display_name: display_name.to_string() },
                &project_ids,
            )?;
            let events = crate::utils::seed::random_events((start, end), entity_id, fqdn, count_per_entity);
            let now = std::time::Instant::now();
            self.events.append(events)?;
            tracing::info!("Seeded entity {} in {:?}", entity_id, now.elapsed());
        }

        Ok(())
    }
}
