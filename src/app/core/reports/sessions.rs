use anyhow::Result;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use duckdb::params_from_iter;

use super::shared::SESSION_DURATION_SQL;
use super::DateRange;

/// One visitor group: the Umami-sessions-style row.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    /// Hashed visitor group id (rotates daily server-side; shortened for display by callers)
    pub visitor_group_id: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    /// Session windows: gaps > 30 min start a new visit
    pub visits: i64,
    /// event = 'pageview' rows
    pub views: i64,
    /// non-pageview (custom) events
    pub events: i64,
    pub browser: Option<String>,
    pub platform: Option<String>,
    pub mobile: Option<bool>,
    pub country: Option<String>,
    pub city: Option<String>,
}

/// One event in a visitor timeline, oldest first.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub created_at: DateTime<Utc>,
    pub event: String,
    pub fqdn: Option<String>,
    pub path: Option<String>,
    pub referrer: Option<String>,
}

/// Most-recent visitor groups in range, newest first. Capped by `limit`.
pub fn session_list_report(
    conn: &DuckDBConn,
    entities: &[String],
    range: &DateRange,
    limit: usize,
) -> Result<Vec<SessionRow>> {
    if entities.is_empty() {
        return Ok(Vec::new());
    }
    let entity_vars = repeat_vars(entities.len());
    let mut params = ParamVec::new();
    params.push(range.start);
    params.push(range.end);
    params.extend(entities);

    let mut stmt = conn.prepare_cached(&format!(
        "--sql
        select
            visitor_group_id,
            min(created_at) as first_seen,
            max(created_at) as last_seen,
            sum(case when time_from_last_event is null or time_from_last_event > {SESSION_DURATION_SQL} then 1 else 0 end) as visits,
            sum(case when event = 'pageview' then 1 else 0 end) as views,
            sum(case when event <> 'pageview' then 1 else 0 end) as events,
            mode(browser) as browser,
            mode(platform) as platform,
            mode(mobile) as mobile,
            mode(country) as country,
            mode(city) as city
        from events
        where
            created_at >= ?::timestamp and created_at < ?::timestamp and
            entity_id in ({entity_vars})
        group by visitor_group_id
        order by last_seen desc
        limit {limit};
        "
    ))?;

    let rows = stmt.query_map(params_from_iter(params), |row| {
        Ok(SessionRow {
            visitor_group_id: row.get(0)?,
            first_seen: row.get(1)?,
            last_seen: row.get(2)?,
            visits: row.get(3)?,
            views: row.get(4)?,
            events: row.get(5)?,
            browser: row.get(6)?,
            platform: row.get(7)?,
            mobile: row.get(8)?,
            country: row.get(9)?,
            city: row.get(10)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, duckdb::Error>>().map_err(anyhow::Error::from)
}

/// Timeline for one visitor group in range, oldest first. Capped by `limit`.
pub fn session_timeline_report(
    conn: &DuckDBConn,
    entities: &[String],
    visitor_group_id: &str,
    range: &DateRange,
    limit: usize,
) -> Result<Vec<SessionEvent>> {
    if entities.is_empty() {
        return Ok(Vec::new());
    }
    let entity_vars = repeat_vars(entities.len());
    let mut params = ParamVec::new();
    params.push(visitor_group_id);
    params.push(range.start);
    params.push(range.end);
    params.extend(entities);

    let mut stmt = conn.prepare_cached(&format!(
        "--sql
        select created_at, event, fqdn, path, referrer
        from events
        where
            visitor_group_id = ?::text and
            created_at >= ?::timestamp and created_at < ?::timestamp and
            entity_id in ({entity_vars})
        order by created_at asc
        limit {limit};
        "
    ))?;

    let rows = stmt.query_map(params_from_iter(params), |row| {
        Ok(SessionEvent {
            created_at: row.get(0)?,
            event: row.get(1)?,
            fqdn: row.get(2)?,
            path: row.get(3)?,
            referrer: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, duckdb::Error>>().map_err(anyhow::Error::from)
}
