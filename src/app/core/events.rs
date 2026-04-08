use std::sync::Arc;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use chrono::{DateTime, Utc};
use rand::distr::{SampleString, StandardUniform};
use tokio::sync::mpsc::Receiver;

use crate::app::models::{Event, event_params};
use crate::app::{DuckDBPool, SqlitePool};

#[derive(Clone)]
pub struct LiwanEvents {
    duckdb: DuckDBPool,
    sqlite: SqlitePool,
    daily_salt: Arc<ArcSwap<(String, DateTime<Utc>)>>,
}

impl LiwanEvents {
    pub fn try_new(duckdb: DuckDBPool, sqlite: SqlitePool) -> Result<Self> {
        let daily_salt: (String, DateTime<Utc>) = {
            tracing::debug!("Loading daily salt");
            sqlite.get()?.query_row("select salt, updated_at from salts where id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };
        Ok(Self { duckdb, sqlite, daily_salt: ArcSwap::new(daily_salt.into()).into() })
    }

    /// Get the daily salt, generating a new one if the current one is older than 24 hours
    pub fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = &**self.daily_salt.load();

        // if the salt is older than 24 hours, replace it with a new one (utils::generate_salt)
        if (Utc::now() - updated_at) > chrono::Duration::hours(24) {
            tracing::debug!("Daily salt expired, generating a new one");
            let new_salt = StandardUniform.sample_string(&mut rand::rng(), 16);
            let now = Utc::now();
            let conn = self.sqlite.get()?;
            conn.execute("update salts set salt = ?, updated_at = ? where id = 1", rusqlite::params![&new_salt, now])?;
            self.daily_salt.store((new_salt.clone(), now).into());
            Ok(new_salt)
        } else {
            Ok(salt.clone())
        }
    }

    /// Append events in batch
    pub fn append(&self, events: impl Iterator<Item = Event>) -> Result<()> {
        let conn = self.duckdb.get()?;
        let mut appender = conn.appender("events").context("Failed to get DuckDB appender")?;
        let mut first_event_time = Utc::now();
        for event in events {
            if event.created_at < first_event_time {
                first_event_time = event.created_at;
            }
            appender.append_row(event_params![event]).context("Failed to append event to DuckDB")?;
        }
        appender.flush().context("Failed to flush events to DuckDB")?;
        update_event_times(&conn, first_event_time).context("Failed to update event times in DuckDB")?;
        Ok(())
    }

    /// Start processing events from the given channel. Blocks until the channel is closed.
    pub async fn process(&self, mut events: Receiver<Event>) -> Result<()> {
        let mut buffer = Vec::with_capacity(1024);

        loop {
            let count = events.recv_many(&mut buffer, 1024).await;

            if count == 0 {
                tracing::info!("Event channel closed, stopping event processing");
                break Ok(());
            }

            let first_event_time = buffer.first().map(|e| e.created_at).unwrap_or_else(Utc::now);
            let events = std::mem::take(&mut buffer).into_iter();

            let conn = self.duckdb.clone();
            let res = tokio::task::spawn_blocking(move || {
                let conn = conn.get().context("Failed to get DuckDB connection")?;
                let mut appender = conn.appender("events").context("Failed to get DuckDB appender")?;
                for event in events {
                    appender.append_row(event_params![event]).context("Failed to append event to DuckDB")?;
                }
                appender.flush().context("Failed to flush events to DuckDB")?;
                update_event_times(&conn, first_event_time)?;
                anyhow::Ok(())
            })
            .await?;

            match res {
                Err(err) => tracing::error!("Event processing task panicked: {:?}", err),
                _ => tracing::debug!("Processed {} events", count),
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
