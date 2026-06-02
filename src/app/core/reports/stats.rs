use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use anyhow::Result;
use chrono::{DateTime, Utc};
use duckdb::params_from_iter;

use super::shared::{build_filter_clause, metric_aggregate_sql};
use super::{DateRange, DimensionFilter, Metric, ReportStats};

/// Return the earliest event timestamp for the selected entities
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

/// Count visitor groups active in the last five minutes
pub fn online_users(conn: &DuckDBConn, entities: &[String]) -> Result<u64> {
    if entities.is_empty() {
        return Ok(0);
    }

    let vars = repeat_vars(entities.len());
    let query = format!(
        "--sql
			select count(distinct e.visitor_group_id)
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

/// Build overall stats for a report range
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
    let (filters_sql, filters_params) = build_filter_clause(filters)?;

    let metric_total = metric_aggregate_sql(Metric::Views, "sd");
    let metric_unique_visitors = metric_aggregate_sql(Metric::UniqueVisitors, "sd");
    let metric_bounce_rate = metric_aggregate_sql(Metric::BounceRate, "sd");
    let metric_avg_time_on_site = metric_aggregate_sql(Metric::AvgTimeOnSite, "sd");

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
					e.visitor_group_id,
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
            bounce_rate: row.get(2)?,
            avg_time_on_site: row.get(3)?,
        })
    })?;

    Ok(result)
}
