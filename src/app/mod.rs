use crate::config::Config;
use crate::utils::hash::{generate_salt, onboarding_token, verify_password};
use crate::utils::refinery_duckdb::DuckDBConnection;
use crossbeam::channel::{Receiver, RecvError};
use crossbeam::sync::{ShardedLock, ShardedLockReadGuard};
use duckdb::{params, DuckdbConnectionManager};
use eyre::{bail, Result};
use models::{event_params, Event, Project, User};
use std::collections::BTreeMap;
use std::sync::Arc;
pub mod models;
pub type Conn = r2d2::PooledConnection<DuckdbConnectionManager>;

#[derive(Clone)]
pub struct App {
    pub conn: r2d2::Pool<DuckdbConnectionManager>,
    pub config: Arc<ShardedLock<Config>>,
    pub onboarding: Arc<ShardedLock<Option<String>>>,
    daily_salt: Arc<ShardedLock<(String, chrono::DateTime<chrono::Utc>)>>,
} 

refinery::embed_migrations!("src/migrations");

const EVENT_BATCH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

impl App {
    pub fn try_new(config: Config) -> Result<Self> {
        let pool = DuckdbConnectionManager::memory()?;
        let conn = r2d2::Pool::new(pool)?;

        conn.get()?.execute_batch(
            "--sql
            ATTACH 'liwan-app.db' as app;
            ATTACH 'liwan-events.db' as event_data;
            USE event_data;
            SET enable_fsst_vectors = true;
        ",
        )?;

 
        let mut runner = migrations::runner();
        runner.set_migration_table_name("app.migrations");
        runner.run(&mut DuckDBConnection(conn.get()?))?;

        let onboarding = {
            // if no users exist, set onboarding to a random string
            let conn = conn.get()?;
            let mut stmt = conn.prepare("select 1 from app.users limit 1")?;
            ShardedLock::new(match stmt.exists([])? {
                true => None,
                false => Some(onboarding_token()),
            }).into()
        };
        
        let daily_salt: (String, chrono::DateTime<chrono::Utc>) = {
            conn.get()?.query_row("select salt, updated_at from app.salts where id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };

        Ok(Self { conn, onboarding, config: ShardedLock::new(config).into(), daily_salt: ShardedLock::new(daily_salt).into() })
    } 

    pub fn conn(&self) -> Result<Conn> {
        Ok(self.conn.get()?)
    }

    pub fn config(&self) -> ShardedLockReadGuard<'_, Config> {
        self.config.read().expect("Failed to acquire read lock")
    }

    pub async fn get_salt(&self) -> Result<String> {
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

    pub fn check_login(&self, username: &str, password: &str) -> Result<bool> {
        let hash: String = self.conn()?.query_row(
            "select password_hash from app.users where username = ?",
            params![username],
            |row| row.get(0),
        )?;
        Ok(verify_password(password, &hash).is_ok())
    }

    pub fn project_entities(&self, project_id: &str) -> Result<BTreeMap<String, String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached( 
            "select e.id, e.display_name from app.entities e join app.project_entities pe on e.id = pe.entity_id where pe.project_id = ?",
        )?;
        let entities = stmt.query_map(params![project_id], |row| Ok((row.get("id")?, row.get("display_name")?)))?;
        Ok(entities.collect::<Result<BTreeMap<String, String>, duckdb::Error>>()?)
    }
 
    pub fn project_entity_ids(&self, project_id: &str) -> Result<Vec<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("select entity_id from app.project_entities where project_id = ?")?;
        let entities = stmt.query_map(params![project_id], |row| Ok(row.get("entity_id")?))?;
        Ok(entities.collect::<Result<Vec<String>, duckdb::Error>>()?)
    } 

    pub fn user(&self, username: &str) -> Result<User> {
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare("select username, password_hash, role, projects from app.users where username = ?")?;
        let user = stmt.query_row(params![username], |row| {
            Ok(User {
                username: row.get("username")?,
                password_hash: row.get("password_hash")?,
                role: row.get::<_, String>("role")?.try_into().unwrap_or_default(),
                projects: row.get::<_, String>("projects")?.split(',').map(str::to_string).collect(),
            })
        });
        user.map_err(|_| eyre::eyre!("user not found"))
    }

    pub fn project(&self, id: &str) -> Result<Project> {
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

    pub fn projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("select id, display_name, public, password from app.projects")?;
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

    pub fn entity_exists(&self, id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("select 1 from app.entities where id = ? limit 1")?;
        Ok(stmt.exists(params![id])?)
    }

    pub fn process_events(&self, events: Receiver<Event>) -> Result<()> {
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

    pub fn session_create(
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
    pub fn session_get(&self, session_id: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("select username, expires_at from app.sessions where id = ?")?;
        let (username, expires_at): (String, chrono::DateTime<chrono::Utc>) =
            stmt.query_row(params![session_id], |row| Ok((row.get("username")?, row.get("expires_at")?)))?;
        if expires_at < chrono::Utc::now() {
            return Ok(None);
        }
        Ok(Some(username))
    }

    pub fn session_delete(&self, session_id: &str) -> Result<()> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare_cached("update app.sessions set expires_at = ? where id = ?")?;
        stmt.execute(params![chrono::Utc::now(), session_id])?;
        Ok(())
    }
}
