use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::utils::{validate, TimeExt};
use crate::{app::DuckDBConn, utils::Timestamp};
use cached::proc_macro::cached;
use cached::SizedCache;
use duckdb::params;
use eyre::Result;
use itertools::Itertools;
use poem_openapi::{Enum, Object};

const CACHE_SIZE_OVERALL_STATS: usize = 512;
const CACHE_SIZE_OVERALL_REPORTS: usize = 512;
const CACHE_SIZE_DIMENSION_REPORTS: usize = 512;

#[derive(Debug, Object)]
pub(crate) struct DateRange {
    pub(crate) start: Timestamp,
    pub(crate) end: Timestamp,
}

#[derive(Debug, Enum)]
#[oai(rename_all = "snake_case")]
pub(crate) enum Metric {
    Views,
    Sessions,
    UniqueVisitors,
    AvgViewsPerSession,
    // AvgDuration,
}

#[derive(Debug, Enum)]
#[oai(rename_all = "snake_case")]
pub(crate) enum Dimension {
    Path,
    Fqdn,
    Referrer,
    Platform,
    Browser,
    Mobile,
    Country,
    City,
}

#[derive(Enum, Debug)]
#[oai(rename_all = "snake_case")]
pub(crate) enum FilterType {
    Equal,
    NotEqual,
    Contains,
    NotContains,
    IsNull,
}

pub(crate) type ReportGraph = Vec<u32>;
pub(crate) type ReportTable = BTreeMap<String, u32>;

#[derive(Object, Clone, Debug, Default)]
#[oai(rename_all = "camelCase")]
pub(crate) struct ReportStats {
    total_views: u32,
    total_sessions: u32,
    unique_visitors: u32,
    avg_views_per_session: u32, // 3 decimal places
}

#[derive(Object, Debug)]
#[oai(rename_all = "camelCase")]
pub(crate) struct DimensionFilter {
    dimension: Dimension,
    filter_type: FilterType,
    value: String,
}

fn filter_sql(_filters: &[DimensionFilter]) -> Result<String> {
    Ok("".to_string())
}

fn metric_sql(metric: &Metric) -> Result<String> {
    Ok(match metric {
        Metric::Views => "count(sd.created_at)",
        Metric::UniqueVisitors => "count(distinct sd.visitor_id)",
        Metric::Sessions => "count(distinct sd.visitor_id || '-' || date_trunc('minute', timestamp 'epoch' + interval '1 second' * cast(floor(extract(epoch from created_at) / 1800) * 1800 as bigint)))",
        Metric::AvgViewsPerSession => "count(sd.created_at) / count(distinct sd.visitor_id)",
    }.to_owned())
}

pub(crate) fn online_users(conn: &DuckDBConn, entities: &[&str]) -> Result<u32> {
    if entities.is_empty() {
        return Ok(0);
    }

    // recheck the validity of the entity IDs to be super sure there's no SQL injection
    if !entities.iter().all(|entity| validate::is_valid_id(entity)) {
        return Err(eyre::eyre!("Invalid entity ID"));
    }
    let entities_list = entities.iter().map(|entity| format!("'{}'", entity)).join(", ");

    let query = format!(
        "--sql
            select count(distinct visitor_id) from events
            where
                entity_id in ({entities_list}) and
                created_at >= now() - interval '5 minutes';
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let online_users = rows.collect::<Result<Vec<u32>, duckdb::Error>>()?;
    Ok(online_users[0])
}

#[cached(
    ty = "SizedCache<String, ReportGraph>",
    create = "{ SizedCache::with_size(CACHE_SIZE_OVERALL_REPORTS)}",
    convert = r#"{format!("{:?}:{}:{:?}:{:?}:{:?}", entities, event, range, filters, metric)}"#,
    result = true
)]
pub(crate) fn overall_report(
    conn: &DuckDBConn,
    entities: &[impl AsRef<str> + Debug],
    event: &str,
    range: &DateRange,
    data_points: u32,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportGraph> {
    if entities.is_empty() {
        return Ok(ReportGraph::default());
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
    let params =
        params![range.start.to_time(), range.end.to_time(), data_points, data_points, event, range.end.to_time()];

    match metric {
        Metric::Views | Metric::UniqueVisitors | Metric::Sessions => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<u32>, duckdb::Error>>()?;
            Ok(report_graph)
        }
        Metric::AvgViewsPerSession => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<f64>, duckdb::Error>>()?;
            Ok(report_graph.iter().map(|v| (v * 1000.0).round() as u32).collect())
        }
    }
}

#[cached(
    ty = "SizedCache<String, ReportStats>",
    create = "{ SizedCache::with_size(CACHE_SIZE_OVERALL_STATS)}",
    convert = r#"{format!("{:?}:{}:{:?}:{:?}", entities, event, range, filters)}"#,
    result = true
)]
pub(crate) fn overall_stats(
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
    let params = params![range.start.to_time(), range.end.to_time(), event];

    let result = stmt.query_row(duckdb::params_from_iter(params), |row| {
        Ok(ReportStats {
            total_views: row.get(0)?,
            total_sessions: row.get(1)?,
            unique_visitors: row.get(2)?,
            avg_views_per_session: (row.get::<_, Option<f64>>(3)?.unwrap_or(0.0) * 1000.0).round() as u32,
        })
    })?;

    Ok(result)
}

#[cached(
    ty = "SizedCache<String, ReportTable>",
    create = "{ SizedCache::with_size(CACHE_SIZE_DIMENSION_REPORTS)}",
    convert = r#"{format!("{:?}:{}:{:?}:{:?}:{:?}:{:?}", entities, event, range, dimension, filters, metric)}"#,
    result = true
)]
pub(crate) fn dimension_report(
    conn: &DuckDBConn,
    entities: &[impl AsRef<str> + Debug],
    event: &str,
    range: DateRange,
    dimension: Dimension,
    filters: &[DimensionFilter],
    metric: Metric,
) -> Result<ReportTable> {
    // recheck the validity of the entity IDs to be super sure there's no SQL injection
    if !entities.iter().all(|entity| validate::is_valid_id(entity.as_ref())) {
        return Err(eyre::eyre!("Invalid entity ID"));
    }

    let entities_list = entities.iter().map(|entity| format!("'{}'", entity.as_ref())).join(", ");
    let filters_clause = filter_sql(filters)?;
    let metric_column = metric_sql(&metric)?;
    let (dimension_column, group_by_columns) = match dimension {
        Dimension::Path => ("concat(fqdn, path)", "fqdn, path"),
        Dimension::Fqdn => ("fqdn", "fqdn"),
        Dimension::Referrer => ("referrer", "referrer"),
        Dimension::Platform => ("platform", "platform"),
        Dimension::Browser => ("browser", "browser"),
        Dimension::Mobile => ("mobile::text", "mobile"),
        Dimension::Country => ("country", "country"),
        Dimension::City => ("city", "city"),
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
                    {dimension_column} as dimension_value,
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
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| {
                let dimension_value: String = row.get(0)?;
                let total_metric: u32 = row.get(1)?;
                Ok((dimension_value, total_metric))
            })?;
            let report_table = rows.collect::<Result<BTreeMap<String, u32>, duckdb::Error>>()?;
            Ok(report_table)
        }
        Metric::AvgViewsPerSession => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| {
                let dimension_value: String = row.get(0)?;
                let total_metric: f64 = row.get(1)?;
                Ok((dimension_value, (total_metric * 1000.0).round() as u32))
            })?;
            let report_table = rows.collect::<Result<BTreeMap<String, u32>, duckdb::Error>>()?;
            Ok(report_table)
        }
    }
}
