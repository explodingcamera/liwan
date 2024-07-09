use crate::config::Config;
use crate::utils::hash::{generate_salt, hash_password, onboarding_token, verify_password};
use crate::utils::refinery_duckdb::DuckDBConnection;
use crate::utils::validate::is_valid_username;

use crossbeam::channel::{Receiver, RecvError};
use crossbeam::sync::ShardedLock;
use duckdb::{params, DuckdbConnectionManager};
use eyre::{bail, ContextCompat, Result};
use models::{event_params, Event, Project, User, UserRole};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) mod models;
pub(crate) mod reports;
pub(crate) type Conn = r2d2::PooledConnection<DuckdbConnectionManager>;

#[derive(Clone)]
pub(crate) struct App {
    pub(crate) conn: r2d2::Pool<DuckdbConnectionManager>,
    pub(crate) config: Arc<Config>,
    pub(crate) onboarding: Arc<ShardedLock<Option<String>>>,
    daily_salt: Arc<ShardedLock<(String, chrono::DateTime<chrono::Utc>)>>,
}

refinery::embed_migrations!("src/migrations");

const EVENT_BATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

impl App {
    pub(crate) fn try_new(config: Config) -> Result<Self> {
        let folder = std::path::Path::new(&config.db_dir);
        if !folder.exists() {
            std::fs::create_dir_all(folder)?;
        }

        let app_db = folder.join("liwan-app.db");
        let app_db = app_db.to_str().wrap_err("invalid db path")?;
        let event_db = folder.join("liwan-events.db");
        let event_db = event_db.to_str().wrap_err("invalid db path")?;

        let pool = DuckdbConnectionManager::file(event_db)?;
        let conn = r2d2::Pool::new(pool)?;

        conn.get()?.execute_batch(&format!(
            "--sql
            SET enable_fsst_vectors = true;
            ATTACH '{app_db}' as app;
            "
        ))?;

        let mut runner = migrations::runner();
        runner.set_migration_table_name("app.migrations");
        runner.run(&mut DuckDBConnection(conn.get()?))?;

        // TODO: WAL is a bit buggy for multiple databases, so maybe switch to two seperate duckdb instances
        conn.get()?.execute_batch(&format!(
            "--sql
            CHECKPOINT;
            CHECKPOINT app;
            "
        ))?;

        let onboarding = {
            // if no users exist, set onboarding to a random string
            let conn = conn.get()?;
            let mut stmt = conn.prepare("select 1 from app.users limit 1")?;
            ShardedLock::new(match stmt.exists([])? {
                true => None,
                false => Some(onboarding_token()),
            })
            .into()
        };

        let daily_salt: (String, chrono::DateTime<chrono::Utc>) = {
            conn.get()?.query_row("select salt, updated_at from app.salts where id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };

        Ok(Self { conn, onboarding, config: config.into(), daily_salt: ShardedLock::new(daily_salt).into() })
    }

    pub(crate) fn conn(&self) -> Result<Conn> {
        Ok(self.conn.get()?)
    }
}

// Events
impl App {
    pub(crate) async fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = {
            let salt = self.daily_salt.read().map_err(|_| eyre::eyre!("Failed to acquire read lock"))?;
            salt.clone()
        };

        // if the salt is older than 24 hours, replace it with a new one (utils::generate_salt)
        if chrono::Utc::now() - updated_at > chrono::Duration::hours(24) {
            let new_salt = generate_salt();
            let now = chrono::Utc::now();
            let conn = self.conn()?;
            conn.execute("update salts set salt = ?, updated_at = ? where id = 1", params![&new_salt, now])?;

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
                    let conn = self.conn()?;
                    let mut appender = conn.appender("events")?;
                    appender.append_row(event_params![event])?;

                    // Non-blockingly drain the remaining events in the queue if there are any
                    for event in events.try_iter() {
                        appender.append_row(event_params![event])?;
                    }
                    appender.flush()?;

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
        let hash: String = self.conn()?.query_row(
            "select password_hash from app.users where username = ?",
            params![username],
            |row| row.get(0),
        )?;
        Ok(verify_password(password, &hash).is_ok())
    }

    pub(crate) fn user(&self, username: &str) -> Result<User> {
        let username = username.to_lowercase();
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare("select username, password_hash, role, projects from app.users where username = ?")?;
        let user = stmt.query_row(params![username], |row| {
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
        let conn = self.conn()?;
        let mut stmt = conn.prepare("select username, password_hash, role, projects from app.users")?;
        let users = stmt.query_map([], |row| {
            Ok(User {
                username: row.get("username")?,
                role: row.get::<_, String>("role")?.try_into().unwrap_or_default(),
                projects: row.get::<_, String>("projects")?.split(',').map(str::to_string).collect(),
            })
        })?;

        Ok(users.collect::<Result<Vec<User>, duckdb::Error>>()?)
    }

    pub(crate) fn user_update_password(&self, username: &str, password: &str) -> Result<()> {
        let conn = self.conn()?;
        let password_hash = hash_password(password)?;
        let mut stmt = conn.prepare_cached("update app.users set password_hash = ? where username = ?")?;
        stmt.execute(params![password_hash, username])?;
        Ok(())
    }

    pub(crate) fn user_update(&self, user: &User) -> Result<User> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("update app.users set role = ?, projects = ? where username = ?")?;
        stmt.execute(params![user.role.to_string(), user.projects.join(","), user.username])?;
        Ok(user.clone())
    }

    pub(crate) fn user_create(
        &self,
        username: &str,
        password: &str,
        role: UserRole,
        projects: Vec<String>,
    ) -> Result<()> {
        if !is_valid_username(username) {
            bail!("invalid username");
        }
        let username = username.to_lowercase();
        let password_hash = hash_password(password)?;
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare_cached("insert into app.users (username, password_hash, role, projects) values (?, ?, ?, ?)")?;
        stmt.execute(params![username, password_hash, role.to_string(), projects.join(",")])?;
        Ok(())
    }

    pub(crate) fn user_delete(&self, username: &str) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("delete from app.users where username = ?")?;
        stmt.execute(params![username])?;
        Ok(())
    }
}

// Projects/Entities
impl App {
    /// Get all entities
    pub(crate) fn entities(&self) -> Result<BTreeMap<String, String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("select id, display_name from app.entities")?;
        let entities = stmt.query_map([], |row| Ok((row.get("id")?, row.get("display_name")?)))?;
        Ok(entities.collect::<Result<BTreeMap<String, String>, duckdb::Error>>()?)
    }

    /// Create a new entity
    pub(crate) fn entity_create(&self, entity: &models::Entity) -> Result<models::Entity> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("insert into app.entities (id, display_name) values (?, ?)")?;
        stmt.execute(params![entity.id, entity.display_name])?;
        Ok(entity.clone())
    }

    /// Delete an entity (does not remove associated events)
    pub(crate) fn entity_delete(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "delete from app.entities where id = ?; delete from app.project_entities where entity_id = ?",
        )?;
        stmt.execute(params![id])?;
        Ok(())
    }

    /// Link an entity to a project
    pub(crate) fn project_add_entity(&self, project_id: &str, entity_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("insert into app.project_entities (project_id, entity_id) values (?, ?)")?;
        stmt.execute(params![project_id, entity_id])?;
        Ok(())
    }

    /// Remove an entity from a project (does not delete the entity itself)
    pub(crate) fn project_remove_entity(&self, project_id: &str, entity_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare_cached("delete from app.project_entities where project_id = ? and entity_id = ?")?;
        stmt.execute(params![project_id, entity_id])?;
        Ok(())
    }

    /// Get all entities associated with a project
    pub(crate) fn project_entities(&self, project_id: &str) -> Result<BTreeMap<String, String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "select e.id, e.display_name from app.entities e join app.project_entities pe on e.id = pe.entity_id where pe.project_id = ?",
        )?;
        let entities = stmt.query_map(params![project_id], |row| Ok((row.get("id")?, row.get("display_name")?)))?;
        Ok(entities.collect::<Result<BTreeMap<String, String>, duckdb::Error>>()?)
    }

    /// Get all entity IDs associated with a project
    pub(crate) fn project_entity_ids(&self, project_id: &str) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("select entity_id from app.project_entities where project_id = ?")?;
        let entities = stmt.query_map(params![project_id], |row| row.get("entity_id"))?;
        Ok(entities.collect::<Result<Vec<String>, duckdb::Error>>()?)
    }

    /// Get a project by ID
    pub(crate) fn project(&self, id: &str) -> Result<Project> {
        let conn = self.conn()?;
        let project = conn
            .prepare("select id, display_name, public, password from app.projects where id = ?")?
            .query_row(params![id], |row| {
                Ok(Project {
                    id: row.get("id")?,
                    display_name: row.get("display_name")?,
                    public: row.get("public")?,
                    secret: row.get("secret")?,
                })
            })?;
        Ok(project)
    }

    /// Get all projects
    pub(crate) fn projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("select id, display_name, public, secret from app.projects")?;
        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get("id")?,
                display_name: row.get("display_name")?,
                public: row.get("public")?,
                secret: row.get("secret")?,
            })
        })?;

        Ok(projects.collect::<Result<Vec<Project>, duckdb::Error>>()?)
    }

    /// Create a new project
    pub(crate) fn project_create(&self, project: &Project) -> Result<Project> {
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare_cached("insert into app.projects (id, display_name, public, secret) values (?, ?, ?, ?)")?;
        stmt.execute(params![project.id, project.display_name, project.public, project.secret])?;
        Ok(project.clone())
    }

    /// Update a project
    pub(crate) fn project_update(&self, project: &Project) -> Result<Project> {
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare_cached("update app.projects set display_name = ?, public = ?, secret = ? where id = ?")?;
        stmt.execute(params![project.display_name, project.public, project.secret, project.id])?;
        Ok(project.clone())
    }

    /// remove the project and all associated project_entities
    pub(crate) fn project_delete(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached(
            "delete from app.projects where id = ?; delete from app.project_entities where project_id = ?",
        )?;

        stmt.execute(params![id, id])?;
        Ok(())
    }

    /// Check if an entity exists
    pub(crate) fn entity_exists(&self, id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("select 1 from app.entities where id = ? limit 1")?;
        Ok(stmt.exists(params![id])?)
    }
}

// Sessions
impl App {
    pub(crate) fn session_create(
        &self,
        session_id: &str,
        username: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("insert into app.sessions (id, username, expires_at) values (?, ?, ?)")?;
        stmt.execute(params![session_id, username, expires_at])?;
        Ok(())
    }

    /// Get the username associated with a session ID, if the session is still valid.
    /// Returns None if the session is expired
    pub(crate) fn session_get(&self, session_id: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("select username, expires_at from app.sessions where id = ?")?;
        let (username, expires_at): (String, chrono::DateTime<chrono::Utc>) =
            stmt.query_row(params![session_id], |row| Ok((row.get("username")?, row.get("expires_at")?)))?;
        if expires_at < chrono::Utc::now() {
            return Ok(None);
        }
        Ok(Some(username))
    }

    pub(crate) fn session_delete(&self, session_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("update app.sessions set expires_at = ? where id = ?")?;
        stmt.execute(params![chrono::Utc::now(), session_id])?;
        Ok(())
    }
}
