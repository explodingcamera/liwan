mod core;
mod db;

pub mod models;
pub use core::reports;
pub use core::{ExternalAuthLogin, ExternalAuthProvider, ExternalAuthSettings, ExternalAuthStart, LiwanExternalAuth};
use std::sync::Arc;

use crate::{config::Config, utils::writable::check_directory_writable};

use crate::utils::r2d2_sqlite::SqliteConnectionManager;
use anyhow::{Context, Result};
use core::{
    LiwanApiKeys, LiwanEntities, LiwanEvents, LiwanOnboarding, LiwanProjectSettings, LiwanProjects, LiwanSessions,
    LiwanSettings, LiwanTeams, LiwanUsers,
};
use duckdb::DuckdbConnectionManager;

pub type DuckDBConn = r2d2::PooledConnection<DuckdbConnectionManager>;
pub type DuckDBPool = r2d2::Pool<DuckdbConnectionManager>;
pub type SqlitePool = r2d2::Pool<SqliteConnectionManager>;
pub use core::{ApiKeyAccess, PruneStats};

pub struct Liwan {
    pub(crate) events_pool: DuckDBPool,

    pub events: LiwanEvents,
    pub api_keys: LiwanApiKeys,
    pub users: LiwanUsers,
    pub sessions: LiwanSessions,
    pub external_auth: LiwanExternalAuth,
    pub onboarding: LiwanOnboarding,
    pub entities: LiwanEntities,
    pub projects: LiwanProjects,
    pub teams: LiwanTeams,
    pub settings: LiwanSettings,
    pub project_settings: LiwanProjectSettings,

    #[cfg(feature = "geoip")]
    pub geoip: Arc<core::LiwanGeoIP>,

    pub config: Config,
}

#[rustfmt::skip]
mod embedded {
    pub(super) mod app { refinery::embed_migrations!("src/migrations/app"); }
    pub(super) mod events { refinery::embed_migrations!("src/migrations/events"); }
}

impl Liwan {
    pub fn try_new(config: Config) -> Result<Arc<Self>> {
        tracing::debug!("Initializing app");
        let dir = std::path::Path::new(&config.data_dir);

        if !dir.exists() {
            tracing::debug!(path = config.data_dir, "Creating data directory since it doesn't exist");
            std::fs::create_dir_all(dir).context("Failed to create data directory")?;
        }
        check_directory_writable(dir);

        tracing::debug!("Initializing databases");
        let conn_app = db::init_sqlite(&dir.join("liwan-app.sqlite"), embedded::app::migrations::runner())?;
        let conn_events = db::init_duckdb(
            &dir.join("liwan-events.duckdb"),
            config.duckdb.clone(),
            embedded::events::migrations::runner(),
        )?;

        Self::from_pools(config, conn_app, conn_events)
    }

    pub fn new_memory(config: Config) -> Result<Arc<Self>> {
        tracing::debug!("Initializing app in memory");
        let conn_app = db::init_sqlite_mem(embedded::app::migrations::runner())?;
        let conn_events = db::init_duckdb_mem(embedded::events::migrations::runner())?;

        Self::from_pools(config, conn_app, conn_events)
    }

    fn from_pools(config: Config, conn_app: SqlitePool, conn_events: DuckDBPool) -> Result<Arc<Self>> {
        Ok(Self {
            #[cfg(feature = "geoip")]
            geoip: core::LiwanGeoIP::try_new(config.clone())?.into(),

            events: LiwanEvents::try_new(conn_events.clone(), conn_app.clone(), config.visitor_group_rotation_hour)?,
            api_keys: LiwanApiKeys::new(conn_app.clone()),
            onboarding: LiwanOnboarding::try_new(&conn_app)?,
            sessions: LiwanSessions::new(conn_app.clone()),
            external_auth: LiwanExternalAuth::try_new(
                conn_app.clone(),
                config.public_url("/api/dashboard/auth/external/callback")?,
            )?,
            entities: LiwanEntities::new(conn_app.clone()),
            projects: LiwanProjects::new(conn_app.clone()),
            teams: LiwanTeams::new(conn_app.clone()),
            settings: LiwanSettings::try_new(conn_app.clone())?,
            project_settings: LiwanProjectSettings::new(conn_app.clone()),
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
        if !self.config.disable_ntp_check {
            tokio::task::spawn(async {
                if let Ok(result) = rsntp::AsyncSntpClient::new().synchronize("pool.ntp.org").await {
                    let offset_seconds = result.clock_offset().as_secs_f64();
                    if offset_seconds.abs() >= 5.0 * 60.0 {
                        tracing::warn!(
                            offset_seconds,
                            "System clock is not synchronized; analytics timestamps may be inaccurate"
                        );
                    }
                }
            });
        }
        #[cfg(feature = "geoip")]
        tokio::task::spawn(core::keep_updated(self.geoip.clone()));
    }

    pub fn shutdown(&self) -> Result<()> {
        self.events_pool.get()?.execute("FORCE CHECKPOINT", [])?; // normal checkpoints don't seem to work consistently on shutdown
        tracing::info!("Shutting down");
        Ok(())
    }
}

#[cfg(any(debug_assertions, test))]
impl Liwan {
    pub fn seed_database(&self, count_per_entity: usize) -> Result<()> {
        use chrono::{Days, Utc};
        use models::{AccessPermission, AccessScope, ApiKeyExpiration, UserRole};

        let entities = vec![
            ("entity-1", "Entity 1", "example.com", vec!["public-project".to_string(), "private-project".to_string()]),
            // ("entity-2", "Entity 2", "test.example.com", vec!["private-project".to_string()]),
            // ("entity-3", "Entity 3", "example.org", vec!["public-project".to_string()]),
        ];
        let projects = [("public-project", "Public Project", true), ("private-project", "Private Project", false)];
        let users = [("admin", "admin", UserRole::Admin), ("user", "user", UserRole::User)];

        for (username, password, role) in users {
            self.users.create(username, password, role)?;
        }

        for (project_id, display_name, public) in projects {
            self.projects.create(
                &models::Project {
                    id: project_id.to_string(),
                    display_name: display_name.to_string(),
                    visibility: if public {
                        models::ProjectVisibility::Public
                    } else {
                        models::ProjectVisibility::Private
                    },
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

        let (key, _) = self.api_keys.create(
            "Expired example",
            &AccessScope::Selected(vec!["entity-1".into()]),
            &AccessScope::Selected(vec![]),
            &[AccessPermission::EventsBatch],
            ApiKeyExpiration::SevenDays,
        )?;
        self.api_keys.expire_for_seed(&key.id)?;

        Ok(())
    }
}
