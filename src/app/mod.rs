mod core;
mod db;

pub(crate) mod models;
pub(crate) use core::reports;

use crate::config::Config;

use core::{LiwanEntities, LiwanEvents, LiwanOnboarding, LiwanProjects, LiwanSessions, LiwanUsers};
use duckdb::DuckdbConnectionManager;
use eyre::Result;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;

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
    pub(crate) geoip: Option<core::geoip::LiwanGeoIP>,

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
        let folder = std::path::Path::new(&config.data_dir);
        if !folder.exists() {
            tracing::debug!(path = config.data_dir, "Creating database folder since it doesn't exist");
            std::fs::create_dir_all(folder)?;
        }

        tracing::debug!("Initializing databases");
        let conn_app = db::init_sqlite(&folder.join("liwan-app.sqlite"), embedded::app::migrations::runner())?;
        let conn_events = db::init_duckdb(&folder.join("liwan-events.duckdb"), embedded::events::migrations::runner())?;

        Ok(Self {
            #[cfg(feature = "geoip")]
            geoip: core::geoip::LiwanGeoIP::try_new(config.clone(), conn_app.clone())?,

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

    pub(crate) fn run_background_tasks(&self) {
        core::geoip::keep_updated(self.geoip.clone());
    }
}

#[cfg(debug_assertions)]
impl Liwan {
    pub(crate) fn seed_database(&self) -> Result<()> {
        use models::UserRole;
        use rand::Rng;

        let entities = vec![
            ("entity-1", "Entity 1", "example.com", vec!["public-project".to_string(), "private-project".to_string()]),
            ("entity-2", "Entity 2", "test.example.com", vec!["private-project".to_string()]),
            ("entity-3", "Entity 3", "example.org", vec!["public-project".to_string()]),
        ];
        let projects = [("public-project", "Public Project", true), ("private-project", "Private Project", false)];
        let users = [("admin", "admin", UserRole::Admin), ("user", "user", UserRole::User)];

        for (username, password, role) in users.iter() {
            self.users.create(username, password, *role, &[])?;
        }

        for (project_id, display_name, public) in projects.iter() {
            self.projects.create(
                &models::Project {
                    id: project_id.to_string(),
                    display_name: display_name.to_string(),
                    public: *public,
                    secret: None,
                },
                &[],
            )?;
        }

        let start = chrono::Utc::now().checked_sub_signed(chrono::Duration::days(365)).unwrap();
        let end = chrono::Utc::now();
        for (entity_id, display_name, fqdn, project_ids) in entities.iter() {
            self.entities.create(
                &models::Entity { id: entity_id.to_string(), display_name: display_name.to_string() },
                project_ids,
            )?;
            let events = crate::utils::seed::random_events(
                (start, end),
                entity_id,
                fqdn,
                rand::thread_rng().gen_range(5000..20000),
            );
            self.events.append(events)?;
        }

        Ok(())
    }
}
