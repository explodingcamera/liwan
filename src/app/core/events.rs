use std::sync::Arc;

use chrono::{DateTime, Utc};
use crossbeam_channel::Receiver;
use crossbeam_utils::sync::ShardedLock;
use eyre::{Result, bail};

use crate::app::models::{Event, event_params};
use crate::app::{DuckDBPool, EVENT_BATCH_INTERVAL, SqlitePool};
use crate::utils::hash::generate_salt;

#[derive(Clone)]
pub struct LiwanEvents {
    duckdb: DuckDBPool,
    sqlite: SqlitePool,
    daily_salt: Arc<ShardedLock<(String, DateTime<Utc>)>>,
}

impl LiwanEvents {
    pub fn try_new(duckdb: DuckDBPool, sqlite: SqlitePool) -> Result<Self> {
        let daily_salt: (String, DateTime<Utc>) = {
            tracing::debug!("Loading daily salt");
            sqlite.get()?.query_row("select salt, updated_at from salts where id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };
        Ok(Self { duckdb, sqlite, daily_salt: ShardedLock::new(daily_salt).into() })
    }

    /// Get the daily salt, generating a new one if the current one is older than 24 hours
    pub fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = {
            let salt = self.daily_salt.read().map_err(|_| eyre::eyre!("Failed to acquire read lock"))?;
            salt.clone()
        };

        // if the salt is older than 24 hours, replace it with a new one (utils::generate_salt)
        if (Utc::now() - updated_at) > chrono::Duration::hours(24) {
            tracing::debug!("Daily salt expired, generating a new one");
            let new_salt = generate_salt();
            let now = Utc::now();
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
        let mut first_event_time = Utc::now();
        for event in events {
            appender.append_row(event_params![event])?;
            if first_event_time > event.created_at {
                first_event_time = event.created_at;
            }
        }
        appender.flush()?;
        update_event_times(&conn, first_event_time)?;
        Ok(())
    }

    /// Start processing events from the given channel. Blocks until the channel is closed.
    pub fn process(&self, events: Receiver<Event>) -> Result<()> {
        let conn = self.duckdb.get()?;

        loop {
            match events.recv() {
                Ok(event) => {
                    let mut appender = conn.appender("events")?;
                    let mut first_event_time = event.created_at;
                    appender.append_row(event_params![event])?;

                    // Non-blockingly drain the remaining events in the queue if there are any
                    let mut count = 1;
                    for event in events.try_iter() {
                        appender.append_row(event_params![event])?;
                        count += 1;

                        if first_event_time > event.created_at {
                            first_event_time = event.created_at;
                        }
                    }

                    appender.flush()?;
                    update_event_times(&conn, first_event_time)?;
                    tracing::debug!("Processed {} events", count);

                    // Sleep to allow more events to be received before the next batch
                    std::thread::sleep(EVENT_BATCH_INTERVAL);
                }
                Err(_) => bail!("event channel closed"),
            }
        }
    }
}

use duckdb::{Connection, Result as DuckResult, params};

pub fn update_event_times(conn: &Connection, from_time: DateTime<Utc>) -> DuckResult<()> {
    // this can probably be simplified, sadly the where clause can't contain window functions
    let sql = "--sql
        with
            filtered_events as (
                select *
                from events
                where created_at >= ?::timestamp or visitor_id in (
                    select visitor_id
                    from events
                    where created_at >= now()::timestamp - interval '24 hours' and created_at < ?::timestamp and time_to_next_event is null
                )
            ),
            cte as (
                select
                    visitor_id,
                    created_at,
                    created_at - lag(created_at) over (partition by visitor_id order by created_at) as time_from_last_event,
                    lead(created_at) over (partition by visitor_id order by created_at) - created_at as time_to_next_event
                from filtered_events
            )
        update events
            set
                time_from_last_event = cte.time_from_last_event,
                time_to_next_event = cte.time_to_next_event
            from cte
            where events.visitor_id = cte.visitor_id and events.created_at = cte.created_at;
    ";

    conn.execute(sql, params![&from_time, &from_time])?;
    Ok(())
}
