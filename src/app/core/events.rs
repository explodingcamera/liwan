use std::sync::Arc;

use crossbeam::{channel::Receiver, sync::ShardedLock};
use eyre::{bail, Result};
use time::OffsetDateTime;

use crate::{
    app::{
        models::{event_params, Event},
        DuckDBPool, SqlitePool, EVENT_BATCH_INTERVAL,
    },
    utils::hash::generate_salt,
};

#[derive(Clone)]
pub struct LiwanEvents {
    duckdb: DuckDBPool,
    sqlite: SqlitePool,
    daily_salt: Arc<ShardedLock<(String, OffsetDateTime)>>,
}

impl LiwanEvents {
    pub fn try_new(duckdb: DuckDBPool, sqlite: SqlitePool) -> Result<Self> {
        let daily_salt: (String, OffsetDateTime) = {
            tracing::debug!("Loading daily salt");
            sqlite.get()?.query_row("select salt, updated_at from salts where id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };
        Ok(Self { duckdb, sqlite, daily_salt: ShardedLock::new(daily_salt).into() })
    }

    /// Get the daily salt, generating a new one if the current one is older than 24 hours
    pub async fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = {
            let salt = self.daily_salt.read().map_err(|_| eyre::eyre!("Failed to acquire read lock"))?;
            salt.clone()
        };

        // if the salt is older than 24 hours, replace it with a new one (utils::generate_salt)
        if (OffsetDateTime::now_utc() - updated_at) > time::Duration::hours(24) {
            tracing::debug!("Daily salt expired, generating a new one");
            let new_salt = generate_salt();
            let now = OffsetDateTime::now_utc();
            let conn = self.sqlite.get()?;
            conn.execute("update salts set salt = ?, updated_at = ? where id = 1", rusqlite::params![&new_salt, now])?;

            if let Ok(mut daily_salt) = self.daily_salt.try_write() {
                daily_salt.0.clone_from(&new_salt);
                daily_salt.1 = now;
                return Ok(new_salt);
            }
        }

        Ok(salt)
    }

    /// Append events in batch
    pub fn append(&self, events: impl Iterator<Item = Event>) -> Result<()> {
        let conn = self.duckdb.get()?;
        let mut appender = conn.appender("events")?;
        for event in events {
            appender.append_row(event_params![event])?;
        }
        appender.flush()?;
        Ok(())
    }

    /// Start processing events from the given channel. Blocks until the channel is closed.
    pub fn process(&self, events: Receiver<Event>) -> Result<()> {
        loop {
            match events.recv() {
                Ok(event) => {
                    let conn = self.duckdb.get()?;
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
                Err(_) => bail!("event channel closed"),
            }
        }
    }
}
