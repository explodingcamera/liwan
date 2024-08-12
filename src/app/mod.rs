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

    #[cfg(debug_assertions)]
    pub(crate) fn seed_database(&self) -> Result<()> {
        use models::UserRole;
        use rand::Rng;

        let entities = vec![
            ("entity-1", "Entity 1", "example.com", vec!["public-project".to_string(), "private-project".to_string()]),
            ("entity-2", "Entity 2", "test.example.com", vec!["private-project".to_string()]),
            ("entity-3", "Entity 3", "example.org", vec!["public-project".to_string()]),
        ];
        let projects = vec![("public-project", "Public Project", true), ("private-project", "Private Project", false)];
        let users = vec![("admin", "admin", UserRole::Admin), ("user", "user", UserRole::User)];

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

fn init_duckdb(path: &PathBuf, mut migrations_runner: Runner) -> Result<r2d2::Pool<DuckdbConnectionManager>> {
    let conn = DuckdbConnectionManager::file(path)?;
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

fn init_sqlite(path: &PathBuf, mut migrations_runner: Runner) -> Result<r2d2::Pool<SqliteConnectionManager>> {
    let conn = SqliteConnectionManager::file(path);
    let pool = r2d2::Pool::new(conn)?;
    migrations_runner.set_migration_table_name("migrations");
    migrations_runner.run(&mut RqlConnection(pool.get()?))?;

    {
        let conn = pool.get()?;
        conn.pragma_update(None, "foreign_keys", &"ON")?;
        conn.pragma_update(None, "journal_mode", &"WAL")?;
        conn.pragma_update(None, "synchronous", &"NORMAL")?;
        conn.pragma_update(None, "mmap_size", &"268435456")?;
        conn.pragma_update(None, "journal_size_limit", &"268435456")?;
        conn.pragma_update(None, "cache_size", &"2000")?;
    }

    Ok(pool)
}
