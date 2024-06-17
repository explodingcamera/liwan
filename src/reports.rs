use std::collections::BTreeMap;

use crate::app::Conn;
use crate::utils::validate;
use cached::proc_macro::cached;
use cached::SizedCache;
use chrono::Duration;
use duckdb::params;
use eyre::Result;
use itertools::Itertools;
use serde::Serialize;

const CACHE_SIZE_OVERALL_STATS: usize = 512;
const CACHE_SIZE_OVERALL_REPORTS: usize = 512;
const CACHE_SIZE_DIMENSION_REPORTS: usize = 512;

#[derive(Debug)]
pub struct DateRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub enum Metric {
    Views,
    Sessions, // Sessions (30 minutes of inactivity)
    UniqueVisitors,
    AvgViewsPerVisitor,
    AvgDuration,
}

#[derive(Debug)]
pub enum Dimension {
    Path, // fqdn + path
    Referrer,
    Platform,
    Browser,
    Mobile,
    Country,
    City,
}

#[derive(Debug)]
pub enum FilterType {
    Equal,
    NotEqual,
    Contains,
    NotContains,
    IsNull,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportGraph(Vec<u32>);

#[derive(Clone, Debug)]
pub struct ReportTable(BTreeMap<String, u32>);

#[derive(Clone, Debug)]
pub struct ReportStats {
    total_views: u32,
    total_sessions: u32,
    unique_visitors: u32,
    avg_views_per_visitor: f32,
    avg_duration: u64,
}

#[derive(Debug)]
pub enum DimensionFilter {
    Path(FilterType, String),
    Fqdn(FilterType, String),
    Referrer(FilterType, String),
    Platform(FilterType, String),
    Browser(FilterType, String),
    Mobile(FilterType, bool),
    Country(FilterType, String),
    City(FilterType, String),
}

fn repeat_vars(count: usize) -> String {
    assert_ne!(count, 0);
    let mut s = "?,".repeat(count);
    s.pop();
    s
}

#[cached(
    ty = "SizedCache<String, ReportGraph>",
    create = "{ SizedCache::with_size(CACHE_SIZE_OVERALL_REPORTS)}",
    convert = r#"{format!("{:?}:{}:{:?}:{:?}:{:?}", entities, event, range, filters, metric)}"#,
    result = true
)]
pub fn overall_report(
    conn: &Conn,
    entities: &[&str],
    event: &str,
    range: DateRange,
    data_points: u32,
    filters: &[DimensionFilter],
    metric: Metric,
) -> Result<ReportGraph> {
    // recheck the validity of the entity IDs to be super sure there's no SQL injection
    if !entities.iter().all(|entity| validate::is_valid_id(entity)) {
        return Err(eyre::eyre!("Invalid entity ID"));
    }
    let entities_list = entities.iter().map(|entity| format!("'{}'", entity)).join(", ");
    let filters_clause = "";

    let metric_column = match metric {
        Metric::Views => "COUNT(created_at)",
        Metric::UniqueVisitors => "COUNT(DISTINCT visitor_id)",
        Metric::Sessions => "COUNT(DISTINCT visitor_id || '-' || DATE_TRUNC('minute', TIMESTAMP 'epoch' + INTERVAL '1 second' * CAST(FLOOR(EXTRACT(EPOCH FROM created_at) / 1800) * 1800 AS BIGINT)))",
        Metric::AvgViewsPerVisitor => "COUNT(created_at) / COUNT(DISTINCT visitor_id)",
        Metric::AvgDuration => "SUM(EXTRACT(EPOCH FROM session_duration)) / COUNT(created_at)",
    }.to_string();

    let query = format!("--sql
        WITH
            params AS (
                SELECT 
                    ?::TIMESTAMP AS start_time,
                    ?::TIMESTAMP AS end_time,
                    ?::INT AS num_buckets,
            ),
            time_bins AS (
                SELECT
                    start_time + (i * (end_time - start_time) / num_buckets) AS bin_start,
                    start_time + ((i + 1) * (end_time - start_time) / num_buckets) AS bin_end
                FROM params, generate_series(0, ?::BIGINT - 1) AS s(i)
            ),
            session_data AS (
                SELECT
                    visitor_id,
                    created_at,
                    LEAD(created_at) OVER (PARTITION BY visitor_id ORDER BY created_at) AS next_visit,
                    COALESCE(LEAD(created_at) OVER (PARTITION BY visitor_id ORDER BY created_at) - created_at, INTERVAL '0' SECOND) AS session_duration
                FROM events, params
                WHERE
                    event = ?::TEXT AND
                    created_at >= params.start_time AND
                    created_at <= params.end_time AND
                    entity_id IN ({entities_list})
                    {filters_clause}
            ),
            event_bins AS (
                SELECT
                    bin_start,
                    {metric_column} AS metric_value
                FROM time_bins tb
                LEFT JOIN session_data sd
                ON sd.created_at >= tb.bin_start AND sd.created_at < COALESCE(tb.bin_end, ?::TIMESTAMP)
                GROUP BY bin_start
            )
        SELECT tb.bin_start, COALESCE(eb.metric_value, 0) AS metric_value
        FROM time_bins tb
        LEFT JOIN event_bins eb ON tb.bin_start = eb.bin_start
        ORDER BY tb.bin_start;
    ");

    let mut stmt = conn.prepare(&query)?;
    let params = params![range.start, range.end, data_points, data_points, event, range.end];

    match metric {
        Metric::Views | Metric::UniqueVisitors | Metric::Sessions => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<u32>, duckdb::Error>>()?;
            Ok(ReportGraph(report_graph))
        }
        Metric::AvgViewsPerVisitor | Metric::AvgDuration => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<f64>, duckdb::Error>>()?;
            Ok(ReportGraph(report_graph.iter().map(|v| (v * 1000.0).round() as u32).collect()))
        }
    }
}

#[cached(
    ty = "SizedCache<String, ReportStats>",
    create = "{ SizedCache::with_size(CACHE_SIZE_OVERALL_STATS)}",
    convert = r#"{format!("{:?}:{}:{:?}:{:?}:{:?}", entities, event, range, filters, metric)}"#,
    result = true
)]
pub fn overall_stats(
    conn: &Conn,
    entities: &[&str],
    event: &str,
    range: DateRange,
    filters: &[DimensionFilter],
    metric: Metric,
) -> Result<ReportStats> {
    todo!()
}

#[cached(
    ty = "SizedCache<String, ReportTable>",
    create = "{ SizedCache::with_size(CACHE_SIZE_DIMENSION_REPORTS)}",
    convert = r#"{format!("{:?}:{}:{:?}:{:?}:{:?}:{:?}", entities, event, range, dimension, filters, metric)}"#,
    result = true
)]
pub fn dimension_report(
    conn: &Conn,
    entities: &[&str],
    event: &str,
    range: DateRange,
    dimension: Dimension,
    filters: &[DimensionFilter],
    metric: Metric,
) -> Result<ReportTable> {
    todo!()
}
