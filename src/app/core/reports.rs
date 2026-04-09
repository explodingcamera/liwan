use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use duckdb::params_from_iter;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const SESSION_DURATION_SQL: &str = "interval '30 minutes'";

// Metric definitions:
// - Session: contiguous events for a visitor with <= 30 minutes between events.
// - Views: count of matching events.
// - UniqueVisitors: distinct visitor_id count
// - BounceRate: single-event sessions / all sessions, where a session boundary is 30 minutes.
// - AvgTimeOnSite: average time_to_next_event (seconds) for intra-session event transitions (<= 30 minutes).

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Hash, PartialEq, Eq)]
pub struct DateRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl DateRange {
    pub fn prev(&self) -> Self {
        let duration = self.end - self.start;
        Self { start: self.start - duration, end: self.start }
    }

    pub fn ends_in_future(&self) -> bool {
        self.end > Utc::now()
    }

    pub fn duration(&self) -> chrono::Duration {
        self.end - self.start
    }
}

impl Display for DateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    Views,
    UniqueVisitors,
    BounceRate,
    AvgTimeOnSite,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Url,
    UrlEntry,
    UrlExit,
    Fqdn,
    Path,
    Referrer,
    Platform,
    Browser,
    Mobile,
    Country,
    City,
    UtmSource,
    UtmMedium,
    UtmCampaign,
    UtmContent,
    UtmTerm,
    ScreenWidth,
    Orientation,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    // Generic filters
    IsNull,

    // String filters
    Equal,
    Contains,
    StartsWith,
    EndsWith,

    // Boolean filters
    IsTrue,
    IsFalse,
}

pub type ReportGraph = Vec<f64>;
pub type ReportTable = BTreeMap<String, f64>;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReportStats {
    pub total_views: u64,
    pub unique_visitors: u64,
    pub bounce_rate: f64,
    pub avg_time_on_site: f64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct DimensionFilter {
    /// The dimension to filter by
    dimension: Dimension,

    /// The type of filter to apply
    /// Note that some filters may not be applicable to all dimensions
    filter_type: FilterType,

    /// Whether to invert the filter (e.g. not equal, not contains)
    /// Defaults to false
    inversed: Option<bool>,

    /// Whether to filter by the strict value (case-sensitive, exact match)
    strict: Option<bool>,

    /// The value to filter by
    /// For `FilterType::IsNull` this should be `None`
    value: Option<String>,
}

fn filter_sql(filters: &[DimensionFilter]) -> Result<(String, ParamVec<'_>)> {
    let mut params = ParamVec::new();

    if filters.is_empty() {
        return Ok((String::new(), params));
    }

    let filter_clauses = filters
        .iter()
        .map(|filter| {
            let filter_value = match (filter.value.clone(), filter.filter_type, filter.inversed.unwrap_or(false)) {
                (Some(value), filter_type, inversed) => {
                    params.push(value);

                    let strict = filter.strict.unwrap_or(false);

                    let sql = match (filter_type, strict) {
                        (FilterType::Equal, false) => "ilike ?",
                        (FilterType::Equal, true) => "like ?",
                        (FilterType::Contains, false) => "ilike '%' || ? || '%'",
                        (FilterType::Contains, true) => "like '%' || ? || '%'",
                        (FilterType::StartsWith, false) => "ilike ? || '%'",
                        (FilterType::StartsWith, true) => "like ? || '%'",
                        (FilterType::EndsWith, false) => "ilike '%' || ?",
                        (FilterType::EndsWith, true) => "like '%' || ?",
                        _ => bail!("Invalid filter type for value"),
                    };

                    if inversed { format!("not {sql}") } else { sql.to_owned() }
                }
                (None, FilterType::IsNull, false) => "is null".into(),
                (None, FilterType::IsNull, true) => "is not null".into(),
                (None, FilterType::IsTrue, false) => "is true".into(),
                (None, FilterType::IsTrue, true) => "is not true".into(),
                (None, FilterType::IsFalse, false) => "is false".into(),
                (None, FilterType::IsFalse, true) => "is not false".into(),
                _ => bail!("Invalid filter type for value"),
            };

            if filter.dimension == Dimension::Mobile
                && !(filter.filter_type == FilterType::IsFalse || filter.filter_type == FilterType::IsTrue)
            {
                bail!("Invalid filter type for boolean dimension");
            }

            if filter.dimension != Dimension::Mobile
                && (filter.filter_type == FilterType::IsFalse || filter.filter_type == FilterType::IsTrue)
            {
                bail!("Invalid filter type for string dimension");
            }

            Ok(match filter.dimension {
                Dimension::Url => format!("concat(fqdn, path) {filter_value}"),
                Dimension::UrlEntry => format!(
                    "(time_from_last_event is null or time_from_last_event > {SESSION_DURATION_SQL}) and concat(fqdn, path) {filter_value}"
                ),
                Dimension::UrlExit => format!(
                    "(time_to_next_event is null or time_to_next_event > {SESSION_DURATION_SQL}) and concat(fqdn, path) {filter_value}"
                ),
                Dimension::Path => format!("path {filter_value}"),
                Dimension::Fqdn => format!("fqdn {filter_value}"),
                Dimension::Referrer => format!("referrer {filter_value}"),
                Dimension::Platform => format!("platform {filter_value}"),
                Dimension::Browser => format!("browser {filter_value}"),
                Dimension::Mobile => format!("mobile {filter_value}"),
                Dimension::Country => format!("country {filter_value}"),
                Dimension::City => format!("city {filter_value}"),
                Dimension::UtmSource => format!("utm_source {filter_value}"),
                Dimension::UtmMedium => format!("utm_medium {filter_value}"),
                Dimension::UtmCampaign => format!("utm_campaign {filter_value}"),
                Dimension::UtmContent => format!("utm_content {filter_value}"),
                Dimension::UtmTerm => format!("utm_term {filter_value}"),
                Dimension::ScreenWidth => format!("screen_width {filter_value}"),
                Dimension::Orientation => format!("orientation {filter_value}"),
            })
        })
        .collect::<Result<Vec<String>>>()?;

    Ok((format!("and ({})", filter_clauses.join(" and ")), params))
}

fn metric_sql(metric: Metric, alias: &str) -> String {
    match metric {
        Metric::Views => format!("count({alias}.created_at)"),
        Metric::UniqueVisitors => {
            // Count the number of unique visitors as the number of distinct visitor IDs
            format!("count(distinct {alias}.visitor_id)")
        }
        Metric::BounceRate => {
            // total sessions: entry events (no recent previous event)
            // bounce sessions: entries that also have no next event within session duration
            format!(
                "--sql
            coalesce(
                count(*)
                    filter (where ({alias}.time_from_last_event is null or {alias}.time_from_last_event > {SESSION_DURATION_SQL}) and
                                 ({alias}.time_to_next_event is null or {alias}.time_to_next_event > {SESSION_DURATION_SQL}))::double /
                nullif(count(*) filter (where {alias}.time_from_last_event is null or {alias}.time_from_last_event > {SESSION_DURATION_SQL}), 0),
                1
            )
            "
            )
        }
        Metric::AvgTimeOnSite => {
            // avg time_to_next_event where time_to_next_event <= session duration and time_to_next_event is not null
            format!(
                "--sql
            coalesce(avg(extract(epoch from {alias}.time_to_next_event)) filter (where {alias}.time_to_next_event is not null and {alias}.time_to_next_event <= {SESSION_DURATION_SQL}), 0)"
            )
        }
    }
}

pub fn earliest_timestamp(conn: &DuckDBConn, entities: &[String]) -> Result<Option<DateTime<Utc>>> {
    if entities.is_empty() {
        return Ok(None);
    }

    let vars = repeat_vars(entities.len());
    let query = format!(
        "--sql
            select min(e.created_at)
            from events e
            where e.entity_id in ({vars});
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let earliest_timestamp = stmt.query_row(params_from_iter(entities), |row| row.get(0))?;
    Ok(earliest_timestamp)
}

pub fn online_users(conn: &DuckDBConn, entities: &[String]) -> Result<u64> {
    if entities.is_empty() {
        return Ok(0);
    }

    let vars = repeat_vars(entities.len());
    let query = format!(
        "--sql
            select count(distinct e.visitor_id)
            from events e
            where
                e.entity_id in ({vars}) and
                e.created_at >= (now()::timestamp - interval '5 minutes');
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let online_users = stmt.query_row(params_from_iter(entities), |row| row.get(0))?;
    Ok(online_users)
}

pub fn overall_report(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    data_points: u32,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportGraph> {
    if data_points == 0 {
        return Ok(Vec::new());
    }

    if entities.is_empty() {
        return Ok(vec![0.; data_points as usize]);
    }

    let mut params = ParamVec::new();

    let (filters_sql, filters_params) = filter_sql(filters)?;
    let metric_sql = metric_sql(*metric, "sd");

    let entity_vars = repeat_vars(entities.len());

    params.push(range.start);
    params.push(range.end);
    params.push(data_points);
    params.push(event);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let query = format!(
        "--sql
        with
            params as (
                select
                    ?::timestamp as start_time,
                    ?::timestamp as end_time,
                    ?::bigint as num_buckets
            ),
            time_bins as (
                select
                    i as bucket_idx,
                    p.start_time + (i * (p.end_time - p.start_time) / p.num_buckets) as bin_start
                from params p, generate_series(0, p.num_buckets - 1) as s(i)
            ),
            session_data as (
                select
                    e.visitor_id,
                    e.created_at,
                    e.time_from_last_event,
                    e.time_to_next_event
                from events e, params p
                where
                    e.event = ?::text and
                    e.created_at >= p.start_time and e.created_at < p.end_time and
                    e.entity_id in ({entity_vars})
                    {filters_sql}
            ),
            bucketed_events as (
                select
                    greatest(
                        0,
                        least(
                            p.num_buckets - 1, floor((
                            extract(epoch from (sd.created_at - p.start_time)) /
                            nullif(extract(epoch from (p.end_time - p.start_time)), 0)
                            ) * p.num_buckets)::bigint
                        )
                    ) as bucket_idx,
                    sd.visitor_id,
                    sd.created_at,
                    sd.time_from_last_event,
                    sd.time_to_next_event
                from session_data sd, params p
            ),
            event_bins as (
                select
                    sd.bucket_idx,
                    {metric_sql} as metric_value
                from bucketed_events sd
                group by sd.bucket_idx
            )
        select
            tb.bin_start,
            coalesce(eb.metric_value, 0)
        from time_bins tb
        left join event_bins eb on tb.bucket_idx = eb.bucket_idx
        order by tb.bucket_idx;
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;

    match metric {
        Metric::Views | Metric::UniqueVisitors => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<f64>, duckdb::Error>>()?;
            Ok(report_graph)
        }
        Metric::AvgTimeOnSite | Metric::BounceRate => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get::<_, Option<f64>>(1))?;
            let report_graph =
                rows.map(|r| r.map(|v| v.unwrap_or(0.0))).collect::<Result<Vec<f64>, duckdb::Error>>()?;
            Ok(report_graph)
        }
    }
}

pub fn overall_stats(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    filters: &[DimensionFilter],
) -> Result<ReportStats> {
    if entities.is_empty() {
        return Ok(ReportStats::default());
    }

    let entity_vars = repeat_vars(entities.len());
    let (filters_sql, filters_params) = filter_sql(filters)?;

    let metric_total = metric_sql(Metric::Views, "sd");
    let metric_unique_visitors = metric_sql(Metric::UniqueVisitors, "sd");
    let metric_bounce_rate = metric_sql(Metric::BounceRate, "sd");
    let metric_avg_time_on_site = metric_sql(Metric::AvgTimeOnSite, "sd");

    let mut params = ParamVec::new();
    params.push(event);
    params.push(range.start);
    params.push(range.end);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let query = format!(
        "--sql
        with
            session_data as (
                select
                    e.visitor_id,
                    e.created_at,
                    e.time_from_last_event,
                    e.time_to_next_event
                from events e
                where
                    e.event = ?::text and
                    e.created_at >= ?::timestamp and e.created_at < ?::timestamp and
                    e.entity_id in ({entity_vars})
                    {filters_sql}
            )
        select
            {metric_total} as total_views,
            {metric_unique_visitors} as unique_visitors,
            {metric_bounce_rate} as bounce_rate,
            {metric_avg_time_on_site} as avg_time_on_site
        from session_data sd;
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let result = stmt.query_row(duckdb::params_from_iter(params), |row| {
        Ok(ReportStats {
            total_views: row.get(0)?,
            unique_visitors: row.get(1)?,
            bounce_rate: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            avg_time_on_site: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
        })
    })?;

    Ok(result)
}

pub fn dimension_report(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    dimension: &Dimension,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportTable> {
    if entities.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut params = ParamVec::new();
    let entity_vars = repeat_vars(entities.len());
    let (filters_sql, filters_params) = filter_sql(filters)?;

    let metric_column = metric_sql(*metric, "sd");
    let (dimension_column, dimension_scope_sql) = match dimension {
        Dimension::Url => ("concat(fqdn, path)", None),
        Dimension::UrlEntry => (
            "concat(fqdn, path)",
            Some(format!("time_from_last_event is null or time_from_last_event > {SESSION_DURATION_SQL}")),
        ),
        Dimension::UrlExit => (
            "concat(fqdn, path)",
            Some(format!("time_to_next_event is null or time_to_next_event > {SESSION_DURATION_SQL}")),
        ),
        Dimension::Path => ("path", None),
        Dimension::Fqdn => ("fqdn", None),
        Dimension::Referrer => ("referrer", None),
        Dimension::Platform => ("platform", None),
        Dimension::Browser => ("browser", None),
        Dimension::Mobile => ("mobile::text", None),
        Dimension::Country => ("country", None),
        Dimension::City => ("concat(country, city)", None),
        Dimension::UtmSource => ("utm_source", None),
        Dimension::UtmMedium => ("utm_medium", None),
        Dimension::UtmCampaign => ("utm_campaign", None),
        Dimension::UtmContent => ("utm_content", None),
        Dimension::UtmTerm => ("utm_term", None),
        Dimension::ScreenWidth => ("screen_width", None),
        Dimension::Orientation => ("orientation", None),
    };
    let filters_sql = match (filters_sql.is_empty(), dimension_scope_sql) {
        (true, Some(scope)) => format!("and ({scope})"),
        (false, Some(scope)) => format!("{filters_sql} and ({scope})"),
        (_, None) => filters_sql,
    };

    params.push(event);
    params.push(range.start);
    params.push(range.end);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let query = format!(
        "--sql
        with
            session_data as (
                select
                    coalesce({dimension_column}, 'Unknown') as dimension_value,
                    visitor_id,
                    created_at,
                    time_from_last_event,
                    time_to_next_event
                from events sd
                where
                    sd.event = ?::text and
                    sd.created_at >= ?::timestamp and sd.created_at < ?::timestamp and
                    sd.entity_id in ({entity_vars})
                    {filters_sql}
            )
        select
            dimension_value,
            {metric_column} as metric_value
        from session_data sd
        group by dimension_value
        order by metric_value desc;
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;

    match metric {
        Metric::Views | Metric::UniqueVisitors => {
            let rows = stmt.query_map(params_from_iter(params), |row| {
                let dimension_value: String = row.get(0)?;
                Ok((dimension_value, row.get(1)?))
            })?;
            let report_table = rows.collect::<Result<BTreeMap<String, f64>, duckdb::Error>>()?;
            Ok(report_table)
        }
        Metric::AvgTimeOnSite | Metric::BounceRate => {
            let rows = stmt.query_map(params_from_iter(params), |row| {
                let dimension_value: String = row.get(0)?;
                Ok((dimension_value, row.get(1)?))
            })?;
            let report_table = rows.collect::<Result<BTreeMap<String, f64>, duckdb::Error>>()?;
            Ok(report_table)
        }
    }
}
