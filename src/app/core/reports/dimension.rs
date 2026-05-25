use std::collections::BTreeMap;

use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use anyhow::Result;
use duckdb::params_from_iter;

use super::shared::{SESSION_DURATION_SQL, build_filter_clause, metric_aggregate_sql};
use super::{DateRange, Dimension, DimensionFilter, Metric, ReportTable};

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
    let (filters_sql, filters_params) = build_filter_clause(filters)?;

    let metric_column = metric_aggregate_sql(*metric, "sd");
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
					visitor_group_id,
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
