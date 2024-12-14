use super::reports::{dimension_report, overall_report, overall_stats};
use super::reports::{DateRange, Dimension, DimensionFilter, Metric, ReportGraph, ReportStats, ReportTable};

use crate::{app::DuckDBConn, utils::to_sorted};
use eyre::Result;
use quick_cache::sync::Cache;
use std::sync::LazyLock;

// the amount of time to wait for a cache guard to be released before giving up and updating the cache
const CACHE_GUARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

type OverallReportCacheKey = (
    // entities:
    Vec<String>,
    // event:
    String,
    // range:
    DateRange,
    // data_points:
    u32,
    // filters:
    Vec<DimensionFilter>,
    // metric:
    Metric,
);

type OverallStatsCacheKey = (
    // entities:
    Vec<String>,
    // event:
    String,
    // range:
    DateRange,
    // filters:
    Vec<DimensionFilter>,
);

type DimensionCacheKey = (
    // entities:
    Vec<String>,
    // event:
    String,
    // range:
    DateRange,
    // dimension:
    Dimension,
    // filters:
    Vec<DimensionFilter>,
    // metric:
    Metric,
);

static OVERALL_REPORT_CACHE: LazyLock<Cache<OverallReportCacheKey, (time::OffsetDateTime, ReportGraph)>> =
    LazyLock::new(|| Cache::new(1024));

static OVERALL_STATS_CACHE: LazyLock<Cache<OverallStatsCacheKey, (time::OffsetDateTime, ReportStats)>> =
    LazyLock::new(|| Cache::new(1024));

static DIMENSION_CACHE: LazyLock<Cache<DimensionCacheKey, (time::OffsetDateTime, ReportTable)>> =
    LazyLock::new(|| Cache::new(1024));

/// [super::reports::overall_report] with caching
pub fn overall_report_cached(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    data_points: u32,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportGraph> {
    let entities = to_sorted(entities);
    let filters = to_sorted(filters);

    get_or_compute(
        &OVERALL_REPORT_CACHE,
        (entities.clone(), event.to_string(), range.clone(), data_points, filters.clone(), *metric),
        || overall_report(conn, &entities, event, range, data_points, &filters, metric),
        range,
    )
}

/// [super::reports::overall_stats] with caching
pub fn overall_stats_cached(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    filters: &[DimensionFilter],
) -> Result<ReportStats> {
    let entities = to_sorted(entities);
    let filters = to_sorted(filters);

    get_or_compute(
        &OVERALL_STATS_CACHE,
        (entities.clone(), event.to_string(), range.clone(), filters.clone()),
        || overall_stats(conn, &entities, event, range, &filters),
        range,
    )
}

/// [super::reports::dimension_report] with caching
pub fn dimension_report_cached(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    dimension: &Dimension,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportTable> {
    let entities = to_sorted(entities);
    let filters = to_sorted(filters);

    get_or_compute(
        &DIMENSION_CACHE,
        (entities.clone(), event.to_string(), range.clone(), *dimension, filters.clone(), *metric),
        || dimension_report(conn, &entities, event, range, dimension, &filters, metric),
        range,
    )
}

/// Check if a cache entry should be invalidated
fn should_invalidate(range: &DateRange, last_update: time::OffsetDateTime) -> bool {
    if !range.ends_in_future() {
        return false;
    }

    let now = time::OffsetDateTime::now_utc();
    let diff = now - last_update;

    match range.duration().whole_days() {
        0..=6 => diff.whole_minutes() >= 1,
        7..=31 => diff.whole_minutes() > 5,
        32..=365 => diff.whole_minutes() > 30,
        _ => diff.whole_hours() > 1,
    }
}

/// Get a value from the cache, or compute it if it's not present or stale.
///
/// Like quick_cache::sync::Cache::get_or_insert_with, but with a timeout for the guard
/// and invalidation logic to recompute when the data might be stale
fn get_or_compute<T, K, F>(
    cache: &LazyLock<Cache<K, (time::OffsetDateTime, T)>>,
    cache_key: K,
    compute: F,
    range: &DateRange,
) -> Result<T>
where
    K: Clone + Send + Sync + 'static + std::hash::Hash + std::cmp::Eq,
    T: Clone + Send + Sync + 'static,
    F: FnOnce() -> Result<T>,
{
    let cached_value = cache.get_value_or_guard(&cache_key, Some(CACHE_GUARD_TIMEOUT));

    let guard = match cached_value {
        quick_cache::sync::GuardResult::Guard(guard) => Some(guard),
        quick_cache::sync::GuardResult::Timeout => None,
        quick_cache::sync::GuardResult::Value(v) => {
            if !should_invalidate(range, v.0) {
                return Ok(v.1.clone());
            }
            cache.remove(&cache_key);
            None
        }
    };
    let result = compute()?;

    match guard {
        Some(guard) => {
            let _ = guard.insert((time::OffsetDateTime::now_utc(), result.clone()));
        }
        None => {
            cache.insert(cache_key, (time::OffsetDateTime::now_utc(), result.clone()));
        }
    }

    Ok(result)
}
