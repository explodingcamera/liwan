use crate::config::Config;
use crate::utils::hash::{generate_salt, hash_password, onboarding_token, verify_password};
use crate::utils::refinery_duckdb::DuckDBConnection;
use crate::utils::validate::is_valid_username;

use crossbeam::channel::{Receiver, RecvError};
use crossbeam::sync::ShardedLock;
use duckdb::DuckdbConnectionManager;
use eyre::{bail, Result};
use models::{event_params, Event, Project, User, UserRole};
use r2d2_sqlite::SqliteConnectionManager;
use refinery::Runner;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) mod models;
pub(crate) mod reports;

pub(crate) type DuckDBConn = r2d2::PooledConnection<DuckdbConnectionManager>;
pub(crate) type SqliteConn = r2d2::PooledConnection<SqliteConnectionManager>;

#[derive(Clone)]
pub(crate) struct App {
    conn_events: r2d2::Pool<DuckdbConnectionManager>,
    conn_app: r2d2::Pool<SqliteConnectionManager>,

    pub(crate) config: Arc<Config>,
    pub(crate) onboarding: Arc<ShardedLock<Option<String>>>,
    daily_salt: Arc<ShardedLock<(String, chrono::DateTime<chrono::Utc>)>>,
}

#[rustfmt::skip]
mod embedded {
    pub(super) mod app { refinery::embed_migrations!("src/migrations/app"); }
    pub(super) mod events { refinery::embed_migrations!("src/migrations/events"); }
}

const EVENT_BATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

impl App {
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

        let onboarding = {
            tracing::debug!("Checking if an onboarding token needs to be generated");
            let conn = conn_app.get()?;
            let mut stmt = conn.prepare("select 1 from users limit 1")?;
            ShardedLock::new(match stmt.exists([])? {
                true => None,
                false => Some(onboarding_token()),
            })
        };

        let daily_salt: (String, chrono::DateTime<chrono::Utc>) = {
            tracing::debug!("Loading daily salt");
            conn_app.get()?.query_row("select salt, updated_at from salts where id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };

        Ok(Self {
            conn_events,
            conn_app,
            config: config.into(),
            daily_salt: ShardedLock::new(daily_salt).into(),
            onboarding: onboarding.into(),
        })
    }

    pub(crate) fn conn_events(&self) -> Result<DuckDBConn> {
        Ok(self.conn_events.get()?)
    }

    pub(crate) fn conn_app(&self) -> Result<SqliteConn> {
        Ok(self.conn_app.get()?)
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
    migrations_runner.run(pool.get()?.deref_mut())?;
    Ok(pool)
}

// MARK: - Event Processing
impl App {
    pub(crate) async fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = {
            let salt = self.daily_salt.read().map_err(|_| eyre::eyre!("Failed to acquire read lock"))?;
            salt.clone()
        };

        // if the salt is older than 24 hours, replace it with a new one (utils::generate_salt)
        if chrono::Utc::now() - updated_at > chrono::Duration::hours(24) {
            tracing::debug!("Daily salt expired, generating a new one");
            let new_salt = generate_salt();
            let now = chrono::Utc::now();
            let conn = self.conn_app()?;
            conn.execute("update salts set salt = ?, updated_at = ? where id = 1", rusqlite::params![&new_salt, now])?;

            if let Ok(mut daily_salt) = self.daily_salt.try_write() {
                daily_salt.0.clone_from(&new_salt);
                daily_salt.1 = now;
                return Ok(new_salt);
            }
        }

        Ok(salt)
    }

    pub(crate) fn process_events(&self, events: Receiver<Event>) -> Result<()> {
        loop {
            match events.recv() {
                Ok(event) => {
                    let conn = self.conn_events.get()?;
                    let mut appender = conn.appender("events")?;
                    appender.append_row(event_params![event])?;

                    // Non-blockingly drain the remaining events in the queue if there are any
                    let mut count = 1;
                    for event in events.try_iter() {
                        appender.append_row(event_params![event])?;
                        count += 1;
                    }
                    appender.flush()?;
                    tracing::debug!("Processed {} events", count);

                    // Sleep to allow more events to be received before the next batch
                    std::thread::sleep(EVENT_BATCH_INTERVAL);
                }
                Err(RecvError) => bail!("event channel closed"),
            }
        }
    }
}

// Users
impl App {
    pub(crate) fn check_login(&self, username: &str, password: &str) -> Result<bool> {
        let username = username.to_lowercase();
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare("select password_hash from users where username = ?")?;
        let hash: String = stmt.query_row([username], |row| row.get(0))?;
        Ok(verify_password(password, &hash).is_ok())
    }

    pub(crate) fn user(&self, username: &str) -> Result<User> {
        let username = username.to_lowercase();
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare("select username, password_hash, role, projects from users where username = ?")?;
        let user = stmt.query_row([username], |row| {
            Ok(User {
                username: row.get("username")?,
                role: row.get::<_, String>("role")?.try_into().unwrap_or_default(),
                projects: row.get::<_, String>("projects")?.split(',').map(str::to_string).collect(),
            })
        });
        user.map_err(|_| eyre::eyre!("user not found"))
    }

    /// Get all users
    pub(crate) fn users(&self) -> Result<Vec<User>> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare("select username, password_hash, role, projects from users")?;
        let users = stmt.query_map([], |row| {
            Ok(User {
                username: row.get("username")?,
                role: row.get::<_, String>("role")?.try_into().unwrap_or_default(),
                projects: row.get::<_, String>("projects")?.split(',').map(str::to_string).collect(),
            })
        })?;
        Ok(users.collect::<Result<Vec<User>, rusqlite::Error>>()?)
    }

    pub(crate) fn user_update_password(&self, username: &str, password: &str) -> Result<()> {
        let conn = self.conn_app()?;
        let password_hash = hash_password(password)?;
        let mut stmt = conn.prepare_cached("update users set password_hash = ? where username = ?")?;
        stmt.execute([&password_hash, username])?;
        Ok(())
    }

    pub(crate) fn user_update(&self, username: &str, role: UserRole, projects: &[String]) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("update users set role = ?, projects = ? where username = ?")?;
        stmt.execute([&role.to_string(), &projects.join(","), username])?;
        Ok(())
    }

    pub(crate) fn user_create(&self, username: &str, password: &str, role: UserRole, projects: &[&str]) -> Result<()> {
        if !is_valid_username(username) {
            bail!("invalid username");
        }
        let username = username.to_lowercase();
        let password_hash = hash_password(password)?;
        let conn = self.conn_app()?;
        let mut stmt =
            conn.prepare_cached("insert into users (username, password_hash, role, projects) values (?, ?, ?, ?)")?;
        stmt.execute([username, password_hash, role.to_string(), projects.join(",")])?;
        Ok(())
    }

    pub(crate) fn user_delete(&self, username: &str) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("delete from users where username = ?")?;
        stmt.execute([username])?;
        Ok(())
    }
}

// MARK: - Projects/Entities
impl App {
    /// Get all entities
    pub(crate) fn entities(&self) -> Result<Vec<models::Entity>> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare("select id, display_name from entities")?;
        let entities = stmt
            .query_map([], |row| Ok(models::Entity { id: row.get("id")?, display_name: row.get("display_name")? }))?;
        Ok(entities.collect::<Result<Vec<models::Entity>, rusqlite::Error>>()?)
    }

    /// Create a new entity
    pub(crate) fn entity_create(&self, entity: &models::Entity, initial_project: &[String]) -> Result<()> {
        let mut conn = self.conn_app()?;
        let tx = conn.transaction()?;
        tx.execute(
            "insert into entities (id, display_name) values (?, ?)",
            rusqlite::params![entity.id, entity.display_name],
        )?;
        for project_id in initial_project {
            tx.execute(
                "insert into project_entities (project_id, entity_id) values (?, ?)",
                rusqlite::params![project_id, entity.id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete an entity (does not remove associated events)
    pub(crate) fn entity_delete(&self, id: &str) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt =
            conn.prepare_cached("delete from entities where id = ?; delete from project_entities where entity_id = ?")?;
        stmt.execute([id])?;
        Ok(())
    }

    /// Link an entity to a project
    pub(crate) fn project_add_entity(&self, project_id: &str, entity_id: &str) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("insert into project_entities (project_id, entity_id) values (?, ?)")?;
        stmt.execute(rusqlite::params![project_id, entity_id])?;
        Ok(())
    }

    /// Remove an entity from a project (does not delete the entity itself)
    pub(crate) fn project_remove_entity(&self, project_id: &str, entity_id: &str) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("delete from project_entities where project_id = ? and entity_id = ?")?;
        stmt.execute(rusqlite::params![project_id, entity_id])?;
        Ok(())
    }

    /// Get all entities associated with a project
    pub(crate) fn project_entities(&self, project_id: &str) -> Result<Vec<models::Entity>> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached(
            "select e.id, e.display_name from entities e join project_entities pe on e.id = pe.entity_id where pe.project_id = ?",
        )?;
        let entities = stmt.query_map(rusqlite::params![project_id], |row| {
            Ok(models::Entity { id: row.get("id")?, display_name: row.get("display_name")? })
        })?;
        Ok(entities.collect::<Result<Vec<models::Entity>, rusqlite::Error>>()?)
    }

    /// Get all projects associated with an entity
    pub(crate) fn entity_projects(&self, entity_id: &str) -> Result<Vec<Project>> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached(
            "select p.id, p.display_name, p.public, p.secret from projects p join project_entities pe on p.id = pe.project_id where pe.entity_id = ?",
        )?;
        let projects = stmt.query_map(rusqlite::params![entity_id], |row| {
            Ok(Project {
                id: row.get("id")?,
                display_name: row.get("display_name")?,
                public: row.get("public")?,
                secret: row.get("secret")?,
            })
        })?;
        Ok(projects.collect::<Result<Vec<Project>, rusqlite::Error>>()?)
    }

    /// Get all entity IDs associated with a project
    pub(crate) fn project_entity_ids(&self, project_id: &str) -> Result<Vec<String>> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("select entity_id from project_entities where project_id = ?")?;
        let entities = stmt.query_map(rusqlite::params![project_id], |row| row.get("entity_id"))?;
        Ok(entities.collect::<Result<Vec<String>, rusqlite::Error>>()?)
    }

    /// Get a project by ID
    pub(crate) fn project(&self, id: &str) -> Result<Project> {
        let conn = self.conn_app()?;
        let project = conn.prepare("select id, display_name, public, secret from projects where id = ?")?.query_row(
            rusqlite::params![id],
            |row| {
                Ok(Project {
                    id: row.get("id")?,
                    display_name: row.get("display_name")?,
                    public: row.get("public")?,
                    secret: row.get("secret")?,
                })
            },
        )?;
        Ok(project)
    }

    /// Get all projects
    pub(crate) fn projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare("select id, display_name, public, secret from projects")?;
        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get("id")?,
                display_name: row.get("display_name")?,
                public: row.get("public")?,
                secret: row.get("secret")?,
            })
        })?;

        Ok(projects.collect::<Result<Vec<Project>, rusqlite::Error>>()?)
    }

    /// Create a new project
    pub(crate) fn project_create(&self, project: &Project) -> Result<Project> {
        let conn = self.conn_app()?;
        let mut stmt =
            conn.prepare_cached("insert into projects (id, display_name, public, secret) values (?, ?, ?, ?)")?;
        stmt.execute(rusqlite::params![project.id, project.display_name, project.public, project.secret])?;
        Ok(project.clone())
    }

    /// Update a project
    pub(crate) fn project_update(&self, project: &Project) -> Result<Project> {
        let conn = self.conn_app()?;
        let mut stmt =
            conn.prepare_cached("update projects set display_name = ?, public = ?, secret = ? where id = ?")?;
        stmt.execute(rusqlite::params![project.display_name, project.public, project.secret, project.id])?;
        Ok(project.clone())
    }

    /// remove the project and all associated project_entities
    pub(crate) fn project_delete(&self, id: &str) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt = conn
            .prepare_cached("delete from projects where id = ?; delete from project_entities where project_id = ?")?;

        stmt.execute([id, id])?;
        Ok(())
    }

    /// Check if an entity exists
    pub(crate) fn entity_exists(&self, id: &str) -> Result<bool> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("select 1 from entities where id = ? limit 1")?;
        Ok(stmt.exists([id])?)
    }
}

// MARK: - Sessions
impl App {
    pub(crate) fn session_create(
        &self,
        session_id: &str,
        username: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("insert into sessions (id, username, expires_at) values (?, ?, ?)")?;
        stmt.execute(rusqlite::params![session_id, username, expires_at])?;
        Ok(())
    }

    /// Get the username associated with a session ID, if the session is still valid.
    /// Returns None if the session is expired
    pub(crate) fn session_get(&self, session_id: &str) -> Result<Option<String>> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("select username, expires_at from sessions where id = ?")?;
        let (username, expires_at): (String, chrono::DateTime<chrono::Utc>) =
            stmt.query_row([session_id], |row| Ok((row.get("username")?, row.get("expires_at")?)))?;
        if expires_at < chrono::Utc::now() {
            return Ok(None);
        }
        Ok(Some(username))
    }

    pub(crate) fn session_delete(&self, session_id: &str) -> Result<()> {
        let conn = self.conn_app()?;
        let mut stmt = conn.prepare_cached("update sessions set expires_at = ? where id = ?")?;
        stmt.execute(rusqlite::params![chrono::Utc::now(), session_id])?;
        Ok(())
    }
}
