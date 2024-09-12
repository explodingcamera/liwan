use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

use crate::app::DuckDBConn;
use crate::utils::validate;
use duckdb::params;
use eyre::Result;
use itertools::Itertools;
use poem_openapi::{Enum, Object};
use time::OffsetDateTime;

// TODO: more fine-grained caching (e.g. don't cache for short durations/ending in now)
// use cached::proc_macro::cached;
// use cached::SizedCache;
// const CACHE_SIZE_OVERALL_STATS: usize = 512;
// const CACHE_SIZE_OVERALL_REPORTS: usize = 512;
// const CACHE_SIZE_DIMENSION_REPORTS: usize = 512;

#[derive(Object)]
pub struct DateRange {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

impl Display for DateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}

impl DateRange {
    pub fn prev(&self) -> DateRange {
        let duration = self.end - self.start;
        DateRange { start: self.start - duration, end: self.start }
    }
}

#[derive(Debug, Enum)]
#[oai(rename_all = "snake_case")]
pub enum Metric {
    Views,
    Sessions,
    UniqueVisitors,
    AvgViewsPerSession,
    // AvgDuration,
}

#[derive(Debug, Enum)]
#[oai(rename_all = "snake_case")]
pub enum Dimension {
    Url,
    Fqdn,
    Path,
    Referrer,
    Platform,
    Browser,
    Mobile,
    Country,
    City,
}

#[derive(Enum, Debug)]
#[oai(rename_all = "snake_case")]
pub enum FilterType {
    Equal,
    NotEqual,
    Contains,
    NotContains,
    IsNull,
}

pub type ReportGraph = Vec<u64>;
pub type ReportTable = BTreeMap<String, u64>;

#[derive(Object, Clone, Debug, Default)]
#[oai(rename_all = "camelCase")]
pub struct ReportStats {
    pub total_views: u64,
    pub total_sessions: u64,
    pub unique_visitors: u64,
    pub avg_views_per_session: u64, // 3 decimal places
}

#[derive(Object, Debug)]
#[oai(rename_all = "camelCase")]
pub struct DimensionFilter {
    dimension: Dimension,
    filter_type: FilterType,
    value: String,
}

fn filter_sql(_filters: &[DimensionFilter]) -> Result<String> {
    Ok(String::new())
}

fn metric_sql(metric: &Metric) -> Result<String> {
    Ok(match metric {
        Metric::Views => "count(sd.created_at)",
        Metric::UniqueVisitors => "count(distinct sd.visitor_id)",
        Metric::Sessions => "count(distinct sd.visitor_id || '-' || date_trunc('minute', timestamp 'epoch' + interval '1 second' * cast(floor(extract(epoch from created_at) / 1800) * 1800 as bigint)))",
        Metric::AvgViewsPerSession => "count(sd.created_at) / count(distinct sd.visitor_id)",
    }.to_owned())
}

pub fn online_users(conn: &DuckDBConn, entities: &[String]) -> Result<u64> {
    if entities.is_empty() {
        return Ok(0);
    }

    // recheck the validity of the entity IDs to be super sure there's no SQL injection
    if !entities.iter().all(|entity| validate::is_valid_id(entity)) {
        return Err(eyre::eyre!("Invalid entity ID"));
    }
    let entities_list = entities.iter().map(|entity| format!("'{entity}'")).join(", ");

    let query = format!(
        "--sql
            select count(distinct visitor_id) from events
            where
                entity_id in ({entities_list}) and
                created_at >= (now()::timestamp - (interval 5 minute));
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let online_users = rows.collect::<Result<Vec<u64>, duckdb::Error>>()?;
    Ok(online_users[0])
}

// #[cached(
//     ty = "SizedCache<String, ReportGraph>",
//     create = "{ SizedCache::with_size(CACHE_SIZE_OVERALL_REPORTS)}",
//     convert = r#"{format!("{:?}:{}:{}:{:?}:{:?}:{}", entities, event, range, filters, metric, data_points)}"#,
//     result = true
// )]
pub fn overall_report(
    conn: &DuckDBConn,
    entities: &[impl AsRef<str> + Debug],
    event: &str,
    range: &DateRange,
    data_points: u32,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportGraph> {
    if entities.is_empty() {
        return Ok(vec![0; data_points as usize]);
    }

    // recheck the validity of the entity IDs to be super sure there's no SQL injection
    if !entities.iter().all(|entity| validate::is_valid_id(entity.as_ref())) {
        return Err(eyre::eyre!("Invalid entity ID"));
    }

    let entities_list = entities.iter().map(|entity| format!("'{}'", entity.as_ref())).join(", ");
    let filters_clause = filter_sql(filters)?;
    let metric_column = metric_sql(metric)?;

    let query = format!("--sql
        with
            params as (
                select
                    ?::timestamp as start_time,
                    ?::timestamp as end_time,
                    ?::int as num_buckets,
            ),
            time_bins as (
                select
                    start_time + (i * (end_time - start_time) / num_buckets) as bin_start,
                    start_time + ((i + 1) * (end_time - start_time) / num_buckets) as bin_end
                from params, generate_series(0, ?::bigint - 1) as s(i)
            ),
            session_data as (
                select
                    visitor_id,
                    created_at,
                    coalesce(lead(created_at) over (partition by visitor_id order by created_at) - created_at, interval '0' second) as session_duration
                from events, params
                where
                    event = ?::text and
                    created_at >= params.start_time and
                    created_at <= params.end_time and
                    entity_id in ({entities_list})
                    {filters_clause}
            ),
            event_bins as (
                select
                    bin_start,
                    {metric_column} as metric_value
                from
                    time_bins tb
                    left join session_data sd
                    on sd.created_at >= tb.bin_start and sd.created_at < coalesce(tb.bin_end, ?::timestamp)
                group by
                    bin_start
            )
        select
            tb.bin_start,
            coalesce(eb.metric_value, 0) as metric_value
        from
            time_bins tb
            left join event_bins eb on tb.bin_start = eb.bin_start
        order by
            tb.bin_start;
    ");

    let mut stmt = conn.prepare_cached(&query)?;
    let params = params![range.start, range.end, data_points, data_points, event, range.end];

    match metric {
        Metric::Views | Metric::UniqueVisitors | Metric::Sessions => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<u64>, duckdb::Error>>()?;
            Ok(report_graph)
        }
        Metric::AvgViewsPerSession => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<f64>, duckdb::Error>>()?;
            Ok(report_graph.iter().map(|v| (v * 1000.0).round() as u64).collect())
        }
    }
}

// #[cached(
//     ty = "SizedCache<String, ReportStats>",
//     create = "{ SizedCache::with_size(CACHE_SIZE_OVERALL_STATS)}",
//     convert = r#"{format!("{:?}:{}:{}:{:?}", entities, event, range, filters)}"#,
//     result = true
// )]
pub fn overall_stats(
    conn: &DuckDBConn,
    entities: &[impl AsRef<str> + Debug],
    event: &str,
    range: &DateRange,
    filters: &[DimensionFilter],
) -> Result<ReportStats> {
    if entities.is_empty() {
        return Ok(ReportStats::default());
    }

    // recheck the validity of the entity IDs to be super sure there's no SQL injection
    if !entities.iter().all(|entity| validate::is_valid_id(entity.as_ref())) {
        return Err(eyre::eyre!("Invalid entity ID"));
    }
    let entities_list = entities.iter().map(|entity| format!("'{}'", entity.as_ref())).join(", ");
    let filters_clause = filter_sql(filters)?;

    let metric_total = metric_sql(&Metric::Views)?;
    let metric_sessions = metric_sql(&Metric::Sessions)?;
    let metric_unique_visitors = metric_sql(&Metric::UniqueVisitors)?;
    let metric_avg_views_per_visitor = metric_sql(&Metric::AvgViewsPerSession)?;

    let query = format!("--sql
        with
            params as (
                select
                    ?::timestamp as start_time,
                    ?::timestamp as end_time
            ),
            session_data as (
                select
                    visitor_id,
                    created_at,
                    coalesce(lead(created_at) over (partition by visitor_id order by created_at) - created_at, interval '0' second) as session_duration
                from events, params
                where
                    event = ?::text and
                    created_at >= params.start_time and
                    created_at <= params.end_time and
                    entity_id in ({entities_list})
                    {filters_clause}
            )
        select
            {metric_total} as total_views,
            {metric_sessions} as total_sessions,
            {metric_unique_visitors} as unique_visitors,
            {metric_avg_views_per_visitor} as avg_views_per_visitor,
        from
            session_data sd;
    ");

    let mut stmt = conn.prepare_cached(&query)?;
    let params = params![range.start, range.end, event];

    let result = stmt.query_row(duckdb::params_from_iter(params), |row| {
        Ok(ReportStats {
            total_views: row.get(0)?,
            total_sessions: row.get(1)?,
            unique_visitors: row.get(2)?,
            avg_views_per_session: (row.get::<_, Option<f64>>(3)?.unwrap_or(0.0) * 1000.0).round() as u64,
        })
    })?;

    Ok(result)
}

// #[cached(
//     ty = "SizedCache<String, ReportTable>",
//     create = "{ SizedCache::with_size(CACHE_SIZE_DIMENSION_REPORTS)}",
//     convert = r#"{format!("{:?}:{}:{}:{:?}:{:?}:{:?}", entities, event, range, dimension, filters, metric)}"#,
//     result = true
// )]
pub fn dimension_report(
    conn: &DuckDBConn,
    entities: &[impl AsRef<str> + Debug],
    event: &str,
    range: &DateRange,
    dimension: &Dimension,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportTable> {
    // recheck the validity of the entity IDs to be super sure there's no SQL injection
    if !entities.iter().all(|entity| validate::is_valid_id(entity.as_ref())) {
        return Err(eyre::eyre!("Invalid entity ID"));
    }

    let entities_list = entities.iter().map(|entity| format!("'{}'", entity.as_ref())).join(", ");
    let filters_clause = filter_sql(filters)?;
    let metric_column = metric_sql(metric)?;
    let (dimension_column, group_by_columns) = match dimension {
        Dimension::Url => ("concat(fqdn, path)", "fqdn, path"),
        Dimension::Path => ("path", "path"),
        Dimension::Fqdn => ("fqdn", "fqdn"),
        Dimension::Referrer => ("referrer", "referrer"),
        Dimension::Platform => ("platform", "platform"),
        Dimension::Browser => ("browser", "browser"),
        Dimension::Mobile => ("mobile::text", "mobile"),
        Dimension::Country => ("country", "country"),
        Dimension::City => ("concat(country, city)", "country, city"),
    };

    let query = format!("--sql
        with
            params as (
                select
                    ?::timestamp as start_time,
                    ?::timestamp as end_time
            ),
            session_data as (
                select
                    coalesce({dimension_column}, 'Unknown') as dimension_value,
                    visitor_id,
                    created_at,
                    coalesce(lead(created_at) over (partition by visitor_id order by created_at) - created_at, interval '0' second) as session_duration
                from events sd, params
                where
                    sd.event = ?::text and
                    sd.created_at >= params.start_time and
                    sd.created_at <= params.end_time and
                    sd.entity_id in ({entities_list})
                    {filters_clause}
                group by
                    {group_by_columns}, visitor_id, created_at
            )
        select
            dimension_value,
            {metric_column} as metric_value
        from
            session_data sd
        group by
            dimension_value
        order by
            metric_value desc;
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let params = params![range.start, range.end, event];

    match metric {
        Metric::Views | Metric::UniqueVisitors | Metric::Sessions => {
            let rows = stmt.query_map(params, |row| {
                let dimension_value: String = row.get(0)?;
                let total_metric: u64 = row.get(1)?;
                Ok((dimension_value, total_metric))
            })?;
            let report_table = rows.collect::<Result<BTreeMap<String, u64>, duckdb::Error>>()?;
            Ok(report_table)
        }
        Metric::AvgViewsPerSession => {
            let rows = stmt.query_map(params, |row| {
                let dimension_value: String = row.get(0)?;
                let total_metric: f64 = row.get(1)?;
                Ok((dimension_value, (total_metric * 1000.0).round() as u64))
            })?;
            let report_table = rows.collect::<Result<BTreeMap<String, u64>, duckdb::Error>>()?;
            Ok(report_table)
        }
    }
}
