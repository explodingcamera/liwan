use std::sync::Arc;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use chrono::{DateTime, Local, NaiveTime, TimeZone, Utc};
use duckdb::{Connection, Result as DuckResult, params};
use rand::distr::{SampleString, StandardUniform};
use tokio::sync::mpsc::Receiver;

use crate::app::models::{Event, GeoDetail, HistoryMode, ResolvedCollectionSettings, event_params};
use crate::app::{DuckDBPool, SqlitePool};
use crate::utils::duckdb::{ParamVec, repeat_vars};

#[derive(Clone)]
pub struct LiwanEvents {
    duckdb: DuckDBPool,
    sqlite: SqlitePool,
    daily_salt: Arc<ArcSwap<(String, DateTime<Utc>)>>,
    visitor_group_rotation_hour: u8,
}

#[derive(Debug, Clone, Default)]
pub struct PruneStats {
    pub total_events: u64,
    pub deleted_events: u64,
    pub cleared_utm_events: u64,
    pub cleared_geo_events: u64,
    pub cleared_session_events: u64,
}

impl LiwanEvents {
    pub fn try_new(duckdb: DuckDBPool, sqlite: SqlitePool, visitor_group_rotation_hour: u8) -> Result<Self> {
        let daily_salt: (String, DateTime<Utc>) = {
            tracing::debug!("Loading visitor group salt");
            sqlite.get()?.query_row("select salt, updated_at from salts where id = 1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
        };
        Ok(Self { duckdb, sqlite, daily_salt: ArcSwap::new(daily_salt.into()).into(), visitor_group_rotation_hour })
    }

    /// Get the visitor group salt, generating a new one after the daily local rotation time.
    pub fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = &**self.daily_salt.load();

        if should_rotate_salt(*updated_at, self.visitor_group_rotation_hour) {
            tracing::debug!("Visitor group salt expired, generating a new one");
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
        let mut first_event_time = None;
        let mut session_entities = Vec::new();
        let mut appender = conn.appender("events").context("Failed to get DuckDB appender")?;
        for event in events {
            if event.track_sessions {
                if first_event_time.is_none_or(|first_event_time| event.created_at < first_event_time) {
                    first_event_time = Some(event.created_at);
                }
                if !session_entities.contains(&event.entity_id) {
                    session_entities.push(event.entity_id.clone());
                }
            }
            appender.append_row(event_params![event]).context("Failed to append event to DuckDB")?;
        }

        appender.flush().context("Failed to flush events to DuckDB")?;
        if let Some(first_event_time) = first_event_time {
            update_event_times(&conn, first_event_time, &session_entities)
                .context("Failed to update event times in DuckDB")?;
        }
        Ok(())
    }

    /// Start processing events from the given channel. Blocks until the channel is closed.
    pub async fn process_events(&self, events_rx: Receiver<Event>) -> Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let events = self.clone();
        std::thread::spawn(move || {
            let res = events.process_events_sync(events_rx).context("Event processing task failed");
            let _ = tx.send(res);
        });
        rx.await??;
        Ok(())
    }

    fn process_events_sync(&self, mut events: Receiver<Event>) -> Result<()> {
        let mut buffer = Vec::with_capacity(1024);
        let conn = self.duckdb.clone();

        loop {
            let count = events.blocking_recv_many(&mut buffer, 512);
            if count == 0 {
                tracing::info!("Event channel closed, stopping event processing");
                break Ok(());
            }

            let mut first_event_time = None;
            let mut session_entities = Vec::new();
            let mut insert_events = || -> Result<()> {
                let conn = conn.get().context("Failed to get DuckDB connection")?;
                let mut appender = conn.appender("events").context("Failed to get DuckDB appender")?;
                for event in buffer.drain(..count) {
                    if event.track_sessions {
                        if first_event_time.is_none_or(|first_event_time| event.created_at < first_event_time) {
                            first_event_time = Some(event.created_at);
                        }
                        if !session_entities.contains(&event.entity_id) {
                            session_entities.push(event.entity_id.clone());
                        }
                    }
                    appender.append_row(event_params![event]).context("Failed to append event to DuckDB")?;
                }

                appender.flush().context("Failed to flush events to DuckDB")?;
                if let Some(first_event_time) = first_event_time {
                    update_event_times(&conn, first_event_time, &session_entities)?;
                }
                Ok(())
            };

            match insert_events() {
                Err(err) => tracing::error!("Event processing task panicked: {:?}", err),
                _ => tracing::debug!("Processed {} events", count),
            }
        }
    }

    pub fn prune_entity(
        &self,
        entity_id: &str,
        settings: &ResolvedCollectionSettings,
        dry_run: bool,
    ) -> Result<PruneStats> {
        let conn = self.duckdb.get()?;
        let mut stats = PruneStats {
            total_events: count_rows(&conn, "select count(*) from events where entity_id = ?", params![entity_id])?,
            ..Default::default()
        };

        if settings.history_mode == HistoryMode::Days
            && let Some(history_days) = settings.history_days
        {
            let cutoff = Utc::now() - chrono::Duration::days(i64::from(history_days));
            stats.deleted_events = count_rows(
                &conn,
                "select count(*) from events where entity_id = ? and created_at < ?::timestamp",
                params![entity_id, cutoff],
            )?;
            if !dry_run {
                conn.execute(
                    "delete from events where entity_id = ? and created_at < ?::timestamp",
                    params![entity_id, cutoff],
                )?;
            }
        }

        if !settings.track_utm_params {
            let sql = "entity_id = ? and (utm_source is not null or utm_medium is not null or utm_campaign is not null or utm_content is not null or utm_term is not null)";
            stats.cleared_utm_events =
                count_rows(&conn, &format!("select count(*) from events where {sql}"), params![entity_id])?;
            if !dry_run {
                conn.execute(
                    &format!(
                        "update events set utm_source = null, utm_medium = null, utm_campaign = null, utm_content = null, utm_term = null where {sql}"
                    ),
                    params![entity_id],
                )?;
            }
        }

        match settings.track_geo {
            GeoDetail::None => {
                let sql = "entity_id = ? and (country is not null or city is not null)";
                stats.cleared_geo_events =
                    count_rows(&conn, &format!("select count(*) from events where {sql}"), params![entity_id])?;
                if !dry_run {
                    conn.execute(
                        &format!("update events set country = null, city = null where {sql}"),
                        params![entity_id],
                    )?;
                }
            }
            GeoDetail::Country => {
                let sql = "entity_id = ? and city is not null";
                stats.cleared_geo_events =
                    count_rows(&conn, &format!("select count(*) from events where {sql}"), params![entity_id])?;
                if !dry_run {
                    conn.execute(&format!("update events set city = null where {sql}"), params![entity_id])?;
                }
            }
            GeoDetail::City => {}
        }

        if !settings.track_sessions {
            let sql = "entity_id = ? and (time_from_last_event is not null or time_to_next_event is not null)";
            stats.cleared_session_events =
                count_rows(&conn, &format!("select count(*) from events where {sql}"), params![entity_id])?;
            if !dry_run {
                conn.execute(
                    &format!("update events set time_from_last_event = null, time_to_next_event = null where {sql}"),
                    params![entity_id],
                )?;
            }
        }

        Ok(stats)
    }
}

fn should_rotate_salt(updated_at: DateTime<Utc>, rotation_hour: u8) -> bool {
    let now = Local::now();
    let rotation_time = NaiveTime::from_hms_opt(u32::from(rotation_hour.min(23)), 0, 0).expect("valid rotation hour");
    let local_rotation = now.date_naive().and_time(rotation_time);
    let latest_rotation = match Local.from_local_datetime(&local_rotation) {
        chrono::LocalResult::Single(rotation) => rotation,
        chrono::LocalResult::Ambiguous(earlier, later) => earlier.min(later),
        chrono::LocalResult::None => now,
    };
    let latest_rotation =
        if now < latest_rotation { latest_rotation - chrono::Duration::days(1) } else { latest_rotation };

    updated_at < latest_rotation.with_timezone(&Utc)
}

fn count_rows<P>(conn: &Connection, sql: &str, params: P) -> DuckResult<u64>
where
    P: duckdb::Params,
{
    conn.query_row(sql, params, |row| row.get(0))
}

pub fn update_event_times(conn: &Connection, from_time: DateTime<Utc>, entities: &[String]) -> DuckResult<()> {
    if entities.is_empty() {
        return Ok(());
    }

    let entity_vars = repeat_vars(entities.len());
    // this can probably be simplified, sadly the where clause can't contain window functions
    let sql = format!("--sql
        with
            filtered_events as (
                select *
                from events
                where entity_id in ({entity_vars}) and (created_at >= ?::timestamp or visitor_group_id in (
                    select visitor_group_id
                    from events
                    where entity_id in ({entity_vars}) and created_at >= now()::timestamp - interval '24 hours' and created_at < ?::timestamp and time_to_next_event is null
                ))
            ),
            cte as (
                select
                    visitor_group_id,
                    created_at,
                    created_at - lag(created_at) over (partition by visitor_group_id order by created_at) as time_from_last_event,
                    lead(created_at) over (partition by visitor_group_id order by created_at) - created_at as time_to_next_event
                from filtered_events
            )
        update events
            set
                time_from_last_event = cte.time_from_last_event,
                time_to_next_event = cte.time_to_next_event
            from cte
            where events.visitor_group_id = cte.visitor_group_id and events.created_at = cte.created_at;
    ");

    let mut params = ParamVec::new();
    params.extend(entities);
    params.push(from_time);
    params.extend(entities);
    params.push(from_time);
    conn.execute(&sql, duckdb::params_from_iter(params))?;
    Ok(())
}
