use crate::config::{Config, Entity};
use crate::utils::hash::generate_salt;
use crossbeam::channel::Receiver;
use crossbeam::sync::ShardedLock;
use duckdb::{params, DuckdbConnectionManager};
use eyre::{bail, Result};
use std::collections::VecDeque;
use std::sync::Arc;

pub type Conn = r2d2::PooledConnection<DuckdbConnectionManager>;

#[derive(Clone)]
pub struct App {
    pub conn: r2d2::Pool<DuckdbConnectionManager>,
    pub config: Arc<Config>,
    daily_salt: Arc<ShardedLock<Salt>>,
}

struct Salt {
    salt: String,
    updated_at: chrono::NaiveDateTime,
}

static LAST_MIGRATION: &str = "2024-06-01-initial";
static MIGRATIONS: &[&str] = &[LAST_MIGRATION];
static CURRENT_SCHEMA: &str = include_str!("./migrations/current.sql");

#[derive(Debug, Clone)]
pub struct Event {
    pub entity_id: String,
    pub visitor_id: String,
    pub event: String,
    pub created_at: chrono::NaiveDateTime,
    pub fqdn: Option<String>,
    pub path: Option<String>,
    pub referrer: Option<String>,
    pub platform: Option<String>,
    pub browser: Option<String>,
    pub mobile: Option<bool>,
    pub country: Option<String>,
    pub city: Option<String>,
}

const BATCH_SIZE: usize = 100;
const FLUSH_TIMEOUT: i64 = 5;

impl App {
    pub async fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = {
            let salt = self.daily_salt.read().map_err(|_| eyre::eyre!("Failed to acquire read lock"))?;
            (salt.salt.clone(), salt.updated_at)
        };

        // if the salt is older than 24 hours, replace it with a new one (utils::generate_salt)
        if chrono::Utc::now().naive_utc() - updated_at > chrono::Duration::hours(24) {
            let new_salt = generate_salt();
            let now = chrono::Utc::now().naive_utc();
            let conn = self.conn()?;
            conn.execute("UPDATE salts SET salt = ?, updated_at = ? WHERE id = 1", params![&new_salt, now])?;

            {
                if let Ok(mut daily_salt) = self.daily_salt.try_write() {
                    daily_salt.salt.clone_from(&new_salt);
                    daily_salt.updated_at = now;
                    return Ok(new_salt);
                }
            }
        }

        Ok(salt)
    }

    pub fn new(config: Config) -> Result<Self> {
        let pool = DuckdbConnectionManager::file(&config.db_path)?;
        let conn = r2d2::Pool::new(pool)?;
        init_db(&conn.get()?)?;

        let (salt, updated_at): (String, chrono::NaiveDateTime) = {
            conn.get()?.query_row("SELECT salt, updated_at FROM salts WHERE id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };

        Ok(Self {
            conn,
            config: Arc::new(config),
            daily_salt: Arc::new(ShardedLock::new(Salt { salt, updated_at })),
        })
    }

    pub fn resolve_entity(&self, id: &str) -> Option<Entity> {
        self.config.entities.iter().find(|&entity| entity.id == id).cloned()
    }

    pub fn process_events(&self, events: Receiver<Event>) -> Result<()> {
        let mut queue = VecDeque::new();
        let mut last_flush = chrono::Utc::now();

        for event in events {
            println!("Received event: {:?}", event);
            queue.push_back(event);

            if queue.len() >= BATCH_SIZE
                || chrono::Utc::now() - last_flush > chrono::Duration::seconds(FLUSH_TIMEOUT)
            {
                println!("Flushing {} events", queue.len());
                self.append_events(queue.drain(..))?;
                last_flush = chrono::Utc::now();
            }
        }

        bail!("event channel closed")
    }

    fn append_events(&self, events: impl Iterator<Item = Event>) -> Result<()> {
        let conn = self.conn()?;
        let mut appender = conn.appender("events")?;
        for event in events {
            appender.append_row(params![
                event.entity_id,
                event.visitor_id,
                event.event,
                event.created_at,
                event.fqdn,
                event.path,
                event.referrer,
                event.platform,
                event.browser,
                event.mobile,
                event.country,
                event.city,
            ])?;
        }
        appender.flush()?;
        Ok(())
    }

    pub fn conn(&self) -> Result<Conn> {
        Ok(self.conn.get()?)
    }
}

fn init_db(conn: &Conn) -> Result<()> {
    //  get the last entry in the migrations table
    let last_migration_exists: Option<String> =
        conn.query_row("SELECT name FROM migrations ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).ok();

    if let Some(last_migration) = last_migration_exists {
        if last_migration == LAST_MIGRATION {
            return Ok(());
        }
        return apply_migrations(conn, &last_migration);
    }

    // apply the latest schema
    conn.execute_batch(CURRENT_SCHEMA)?;
    Ok(())
}

fn apply_migrations(_conn: &Conn, last_migration: &str) -> Result<()> {
    let _last_migration_index = MIGRATIONS
        .iter()
        .position(|&migration| migration == last_migration)
        .ok_or_else(|| eyre::eyre!("Unknown migration: {}", last_migration))?;

    bail!("Migrations not implemented");
}
