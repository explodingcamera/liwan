use std::collections::BTreeMap;

use crate::app::Conn;
use cached::proc_macro::cached;
use cached::SizedCache;
use eyre::Result;

const CACHE_SIZE_OVERALL_STATS: usize = 512;
const CACHE_SIZE_OVERALL_REPORTS: usize = 512;
const CACHE_SIZE_DIMENSION_REPORTS: usize = 512;

#[derive(Debug)]
pub struct DateRange {
    start: chrono::NaiveDateTime,
    end: chrono::NaiveDateTime,
    points: u32,
}

#[derive(Debug)]
pub enum Metric {
    Views,
    Sessions, // Sessions (30 minutes of inactivity)
    UniqueVisitors,
    ViewsPerVisitor,
    AverageDuration,
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

#[derive(Clone, Debug)]
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
    filters: &[DimensionFilter],
    metric: Metric,
) -> Result<ReportGraph> {
    todo!()
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
    convert = r#"{format!("{:?}:{}:{:?}:{:?}:{:?}:{:?}", entities, event, range, dimensions, filters, metric)}"#,
    result = true
)]
pub fn dimension_report(
    conn: &Conn,
    entities: &[&str],
    event: &str,
    range: DateRange,
    dimensions: &[Dimension],
    filters: &[DimensionFilter],
    metric: Metric,
) -> Result<ReportTable> {
    todo!()
}
