use crate::config::{Config, Entity};
use crossbeam_channel::Receiver;
use duckdb::{params, DuckdbConnectionManager};
use eyre::{bail, Result};
use r2d2::PooledConnection;
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone)]
pub struct App {
    pub conn: r2d2::Pool<DuckdbConnectionManager>,
    pub config: Arc<Config>,
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
    pub fn new(config: Config) -> Result<Self> {
        let pool = DuckdbConnectionManager::file(&config.db_path)?;
        let conn = r2d2::Pool::new(pool)?;
        Ok(Self { conn, config: Arc::new(config) })
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

            if queue.len() >= BATCH_SIZE || chrono::Utc::now() - last_flush > chrono::Duration::seconds(FLUSH_TIMEOUT) {
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

    fn conn(&self) -> Result<PooledConnection<DuckdbConnectionManager>> {
        Ok(self.conn.get()?)
    }

    pub fn apply_migrations(&self, last_migration: &str) -> Result<()> {
        let _last_migration_index = MIGRATIONS
            .iter()
            .position(|&migration| migration == last_migration)
            .ok_or_else(|| eyre::eyre!("Unknown migration: {}", last_migration))?;

        bail!("Migrations not implemented");
    }

    pub fn create_tables(&self) -> Result<()> {
        let conn = self.conn()?;

        //  get the last entry in the migrations table
        let last_migration_exists: Option<String> =
            conn.query_row("SELECT name FROM migrations ORDER BY id DESC LIMIT 1", [], |row| row.get(0)).ok();

        if let Some(last_migration) = last_migration_exists {
            if last_migration == LAST_MIGRATION {
                return Ok(());
            }
            return self.apply_migrations(&last_migration);
        }

        // apply the latest schema
        conn.execute_batch(CURRENT_SCHEMA)?;
        Ok(())
    }
}
