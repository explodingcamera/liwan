use std::sync::Arc;

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use chrono::{DateTime, Local, NaiveTime, TimeZone, Utc};
use duckdb::{Connection, Result as DuckResult, params};
use futures_lite::{StreamExt, future};
use rand::distr::{SampleString, StandardUniform};
use tokio::sync::mpsc::Receiver;
use tokio_util::time::DelayQueue;

use crate::app::models::{Event, EventExit, GeoDetail, ResolvedCollectionSettings, event_params};
use crate::app::{DuckDBPool, SqlitePool};
use crate::utils::duckdb::{ParamVec, repeat_vars};

const EVENT_EXIT_DELAY: std::time::Duration = std::time::Duration::from_secs(30);

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

    /// Get the visitor group salt, generating a new one after the daily local rotation time
    pub fn get_salt(&self) -> Result<String> {
        let (salt, updated_at) = &**self.daily_salt.load();

        if should_rotate_salt(*updated_at, self.visitor_group_rotation_hour) {
            tracing::debug!("Visitor group salt expired, generating a new one");
            let new_salt = StandardUniform.sample_string(&mut rand::rng(), 16);
            let now = Utc::now();
            let conn = self.sqlite.get()?;
            conn.execute(
                "update salts set salt = :salt, updated_at = :updated_at where id = 1",
                rusqlite::named_params! { ":salt": &new_salt, ":updated_at": now },
            )?;
            self.daily_salt.store((new_salt.clone(), now).into());
            Ok(new_salt)
        } else {
            Ok(salt.clone())
        }
    }

    /// Append events in a batch and update session timing fields when needed
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

    /// Start processing events from the given channel. Blocks until the channel is closed
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

    /// Process exit updates after a short delay so queued events can reach shared storage first.
    pub async fn process_exits(&self, mut exits_rx: Receiver<EventExit>) -> Result<()> {
        let mut pending = DelayQueue::new();
        let mut channel_closed = false;

        loop {
            if channel_closed && pending.is_empty() {
                tracing::info!("Event exit channel closed, stopping event exit processing");
                return Ok(());
            }

            tokio::select! {
                exit = exits_rx.recv(), if !channel_closed => match exit {
                    Some(exit) => {
                        pending.insert(exit, EVENT_EXIT_DELAY);
                    }
                    None => channel_closed = true,
                },
                Some(expired) = pending.next(), if !pending.is_empty() => {
                    let mut exits = vec![expired.into_inner()];
                    while let Some(Some(expired)) = future::poll_once(pending.next()).await {
                        exits.push(expired.into_inner());
                    }

                    let count = exits.len();
                    let events = self.clone();
                    match tokio::task::spawn_blocking(move || events.update_exits(exits)).await? {
                        Ok(matched) => tracing::debug!(count, matched, "Processed event exits"),
                        Err(err) => tracing::error!(?err, "Failed to process event exits"),
                    }
                }
            }
        }
    }

    fn process_events_sync(&self, mut events: Receiver<Event>) -> Result<()> {
        let mut buffer = Vec::with_capacity(512);

        loop {
            let event_count = events.blocking_recv_many(&mut buffer, 512);
            if event_count == 0 {
                tracing::info!("Event channel closed, stopping event processing");
                break Ok(());
            }

            match self.append(buffer.drain(..)) {
                Err(err) => tracing::error!(?err, "Failed to process events"),
                Ok(()) => tracing::debug!(event_count, "Processed events"),
            }
        }
    }

    fn update_exits(&self, exits: Vec<EventExit>) -> Result<usize> {
        let conn = self.duckdb.get().context("Failed to get DuckDB connection")?;
        let mut matched = 0;
        for exit in exits {
            matched += usize::from(update_event_exit(&conn, &exit)?);
        }
        Ok(matched)
    }

    /// Preview or apply collection-setting pruning for a single entity
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

        if let crate::app::models::DataRetention::Days(data_retention_days) = settings.data_retention {
            let cutoff = Utc::now() - chrono::Duration::days(i64::from(data_retention_days.get()));
            stats.deleted_events = count_rows(
                &conn,
                "select count(*) from events where entity_id = $entity_id and created_at < $cutoff::timestamp",
                duckdb::named_params! { "entity_id": entity_id, "cutoff": cutoff },
            )?;
            if !dry_run {
                conn.execute(
                    "delete from events where entity_id = $entity_id and created_at < $cutoff::timestamp",
                    duckdb::named_params! { "entity_id": entity_id, "cutoff": cutoff },
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
            let sql = "entity_id = ? and (time_from_last_event is not null or time_to_next_event is not null or exited_at is not null)";
            stats.cleared_session_events =
                count_rows(&conn, &format!("select count(*) from events where {sql}"), params![entity_id])?;
            if !dry_run {
                conn.execute(
                    &format!(
                        "update events set time_from_last_event = null, time_to_next_event = null, exited_at = null where {sql}"
                    ),
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

fn count_rows(conn: &Connection, sql: &str, params: impl duckdb::Params) -> DuckResult<u64> {
    conn.query_row(sql, params, |row| row.get(0))
}

fn update_event_exit(conn: &Connection, exit: &EventExit) -> DuckResult<bool> {
    let sql = "--sql
        update events
        set exited_at = greatest(
            coalesce(exited_at, created_at),
            least($exited_at::timestamp, created_at + interval '30 minutes')
        )
        where
            entity_id = $entity_id and
            visitor_group_id = $visitor_group_id and
            event = $event and
            fqdn is not distinct from $fqdn and
            path is not distinct from $path and
            time_to_next_event is null and
            created_at = (
                select max(candidate.created_at)
                from events candidate
                where
                    candidate.entity_id = $entity_id and
                    candidate.visitor_group_id = $visitor_group_id and
                    candidate.event = $event and
                    candidate.fqdn is not distinct from $fqdn and
                    candidate.path is not distinct from $path and
                    candidate.time_to_next_event is null and
                    candidate.created_at <= $exited_at::timestamp and
                    candidate.created_at >= $exited_at::timestamp - interval '30 minutes'
            )";
    let updated = conn.execute(
        sql,
        duckdb::named_params! {
            "entity_id": &exit.entity_id,
            "visitor_group_id": &exit.visitor_group_id,
            "event": &exit.event,
            "fqdn": &exit.fqdn,
            "path": &exit.path,
            "exited_at": exit.created_at,
        },
    )?;
    Ok(updated > 0)
}

fn update_event_times(conn: &Connection, from_time: DateTime<Utc>, entities: &[String]) -> DuckResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Liwan;
    use crate::config::Config;

    fn event(created_at: DateTime<Utc>) -> Event {
        Event {
            entity_id: "entity-1".to_string(),
            visitor_group_id: "visitor-1".to_string(),
            event: "pageview".to_string(),
            created_at,
            fqdn: Some("example.com".to_string()),
            path: Some("/docs".to_string()),
            referrer: None,
            platform: None,
            browser: None,
            mobile: None,
            country: None,
            city: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            utm_content: None,
            utm_term: None,
            screen_width: None,
            orientation: None,
            track_sessions: true,
        }
    }

    #[test]
    fn exit_updates_only_the_latest_event_without_a_next_event() {
        let app = Liwan::new_memory(Config::default()).expect("failed to create app");
        let first = Utc::now() - chrono::Duration::minutes(2);
        let second = first + chrono::Duration::minutes(1);
        app.events.append(vec![event(first), event(second)].into_iter()).expect("failed to append events");

        let conn = app.events_conn().expect("failed to get event connection");
        let matched = update_event_exit(
            &conn,
            &EventExit {
                entity_id: "entity-1".to_string(),
                visitor_group_id: "visitor-1".to_string(),
                event: "pageview".to_string(),
                created_at: second + chrono::Duration::seconds(15),
                fqdn: Some("example.com".to_string()),
                path: Some("/docs".to_string()),
            },
        )
        .expect("failed to update exit");

        assert!(matched);
        let rows = conn
            .prepare("select exited_at from events order by created_at")
            .expect("failed to prepare query")
            .query_map([], |row| row.get::<_, Option<DateTime<Utc>>>(0))
            .expect("failed to query exits")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("failed to collect exits");
        assert_eq!(
            rows[1].map(|value| value.timestamp_millis()),
            Some((second + chrono::Duration::seconds(15)).timestamp_millis())
        );
        assert_eq!(rows[0], None);
    }
}
