use std::collections::BTreeMap;

use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use anyhow::Result;
use duckdb::params_from_iter;

use super::shared::{
    SESSION_DURATION_SQL, build_filter_clause, metric_aggregate_sql, metric_sample_count_sql, range_events_sql,
};
use super::{DateRange, Dimension, DimensionFilter, Metric, ReportTable};

/// Select top dimensions per range and return their values across every requested range.
pub fn dimension_report(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    ranges: &[DateRange],
    dimension: &Dimension,
    property_key: Option<&str>,
    filters: &[DimensionFilter],
    metric: &Metric,
    max_results: usize,
    min_samples: u64,
) -> Result<Vec<ReportTable>> {
    let mut reports = vec![BTreeMap::new(); ranges.len()];
    if entities.is_empty() || ranges.is_empty() {
        return Ok(reports);
    }

    let mut params = ParamVec::new();
    let range_events = range_events_sql(ranges, &mut params);
    let range_count = ranges.len();
    let entity_vars = repeat_vars(entities.len());
    let (filters_sql, filters_params) = build_filter_clause(filters)?;

    let metric_column = metric_aggregate_sql(*metric, "sd");
    let sample_count = metric_sample_count_sql(*metric, "sd");
    let sample_filter = if matches!(metric, Metric::BounceRate | Metric::AvgTimeOnSite) {
        format!("where sample_count >= {min_samples}")
    } else {
        String::new()
    };
    let order_column = match metric {
        Metric::BounceRate => "metric_value asc",
        _ => "metric_value desc",
    };
    let value_sql = if matches!(metric, Metric::BounceRate | Metric::AvgTimeOnSite) {
        format!("case when a.sample_count >= {min_samples} then a.metric_value end")
    } else {
        "a.metric_value".to_string()
    };
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
        Dimension::Property => match property_key {
            None => ("unnest(map_keys(properties))", Some("properties is not null".to_string())),
            Some(_) => ("map_extract_value(properties, ?)", Some("map_contains(properties, ?)".to_string())),
        },
    };
    let filters_sql = match (filters_sql.is_empty(), dimension_scope_sql) {
        (true, Some(scope)) => format!("and ({scope})"),
        (false, Some(scope)) => format!("{filters_sql} and ({scope})"),
        (_, None) => filters_sql,
    };

    params.extend(property_key);
    params.push(event);
    params.extend(entities);
    params.extend_from_params(filters_params);
    params.extend(property_key);

    let query = format!(
        "--sql
		with
			range_events as ({range_events}),
			session_data as (
				select
					range_idx,
					coalesce({dimension_column}, 'Unknown') as dimension_value,
					visitor_group_id,
					created_at,
					time_from_last_event,
					time_to_next_event,
					exited_at
				from range_events sd
				where
					sd.event = ?::text and
					sd.entity_id in ({entity_vars})
					{filters_sql}
			),
			aggregates as materialized (
				select range_idx, dimension_value,
					{metric_column} as metric_value, {sample_count} as sample_count
				from session_data sd
				group by range_idx, dimension_value
			),
			selected_keys as (
				select distinct dimension_value from aggregates
				{sample_filter}
				qualify row_number() over (partition by range_idx order by {order_column}, dimension_value) <= {max_results}
			)
		select r.range_idx, k.dimension_value, {value_sql}
		from range({range_count}) r(range_idx)
		cross join selected_keys k
		left join aggregates a on a.range_idx = r.range_idx and a.dimension_value = k.dimension_value;
	"
    );

    let mut stmt = conn.prepare_cached(&query)?;

    let mut rows = stmt.query(params_from_iter(params))?;
    while let Some(row) = rows.next()? {
        reports[row.get::<_, usize>(0)?].insert(row.get(1)?, row.get(2)?);
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Liwan;
    use crate::app::models::FilterType;
    use crate::config::Config;
    use chrono::{Duration, TimeZone, Utc};

    #[test]
    fn dimension_report_limits_results() {
        let app = Liwan::new_memory(Config::default()).expect("failed to create app");
        let max_results = app.config.limits.report_max_dimension_results;
        let conn = app.events_conn().expect("failed to get events conn");
        conn.execute_batch(&format!(
            "insert into events (entity_id, visitor_group_id, event, created_at, fqdn, path)
             select 'entity-1', 'visitor-' || i, 'pageview', '2024-01-01'::timestamp, 'example.com', '/' || i
             from range({}) values(i)",
            max_results + 1
        ))
        .expect("failed to insert events");
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let range = DateRange { start, end: start + Duration::days(1) };
        let ranges = [range];

        let report = dimension_report(
            &conn,
            &["entity-1".to_string()],
            "pageview",
            &ranges,
            &Dimension::Path,
            None,
            &[],
            &Metric::Views,
            max_results,
            0,
        )
        .expect("failed to build dimension report")
        .remove(0);

        assert_eq!(report.len(), max_results);
    }

    #[test]
    fn bounce_rate_reports_entry_session_counts() {
        let app = Liwan::new_memory(Config::default()).expect("failed to create app");
        let conn = app.events_conn().expect("failed to get events conn");
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().expect("valid date");
        for (visitor, path, offset) in
            [("one", "/popular", 0), ("one", "/popular", 1), ("two", "/popular", 2), ("three", "/small", 3)]
        {
            conn.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at, path) values ('a', ?, 'pageview', ?, ?)",
                duckdb::params![visitor, start + Duration::hours(offset), path],
            )
            .expect("failed to insert pageview");
        }
        conn.execute(
            "insert into events (entity_id, visitor_group_id, event, created_at, path, time_to_next_event) values ('a', 'four', 'pageview', ?, '/good', interval '5 minutes')",
            duckdb::params![start + Duration::hours(4)],
        )
        .expect("failed to insert entry pageview");
        conn.execute(
            "insert into events (entity_id, visitor_group_id, event, created_at, path, time_from_last_event) values ('a', 'four', 'pageview', ?, '/good', interval '5 minutes')",
            duckdb::params![start + Duration::hours(4) + Duration::minutes(5)],
        )
        .expect("failed to insert second pageview");
        let range = DateRange { start: start - Duration::hours(1), end: start + Duration::days(1) };
        let ranges = [range];
        let report = dimension_report(
            &conn,
            &["a".into()],
            "pageview",
            &ranges,
            &Dimension::Path,
            None,
            &[],
            &Metric::BounceRate,
            10,
            0,
        )
        .expect("failed to build bounce rate report")
        .remove(0);

        assert_eq!(report["/popular"], Some(1.0));
        assert_eq!(report["/small"], Some(1.0));
        assert_eq!(report["/good"], Some(0.0));

        let ranked = dimension_report(
            &conn,
            &["a".into()],
            "pageview",
            &ranges,
            &Dimension::Path,
            None,
            &[],
            &Metric::BounceRate,
            1,
            1,
        )
        .expect("failed to build ranked bounce rate report")
        .remove(0);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked["/good"], Some(0.0));

        let filtered = dimension_report(
            &conn,
            &["a".into()],
            "pageview",
            &ranges,
            &Dimension::Path,
            None,
            &[],
            &Metric::BounceRate,
            10,
            2,
        )
        .expect("failed to filter bounce rate report")
        .remove(0);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered["/popular"], Some(1.0));
    }

    #[test]
    fn avg_time_reports_only_measured_pageviews() {
        let app = Liwan::new_memory(Config::default()).expect("failed to create app");
        let conn = app.events_conn().expect("failed to get events conn");
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).single().expect("valid date");
        for (visitor, offset, duration) in [("one", 0, Some(60)), ("two", 1, Some(120)), ("three", 2, None)] {
            let created_at = start + Duration::hours(offset);
            conn.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at, path, exited_at) values ('a', ?, 'pageview', ?, '/', ?)",
                duckdb::params![visitor, created_at, duration.map(|seconds| created_at + Duration::seconds(seconds))],
            )
            .expect("failed to insert pageview");
        }
        conn.execute(
            "insert into events (entity_id, visitor_group_id, event, created_at, path, exited_at) values ('a', 'four', 'pageview', ?, '/small', ?)",
            duckdb::params![start + Duration::hours(3), start + Duration::hours(3) + Duration::seconds(600)],
        )
        .expect("failed to insert pageview");
        let range = DateRange { start: start - Duration::hours(1), end: start + Duration::days(1) };
        let ranges = [range];
        let report = dimension_report(
            &conn,
            &["a".into()],
            "pageview",
            &ranges,
            &Dimension::Path,
            None,
            &[],
            &Metric::AvgTimeOnSite,
            10,
            0,
        )
        .expect("failed to build average time report")
        .remove(0);

        assert_eq!(report["/"], Some(90.0));
        assert_eq!(report["/small"], Some(600.0));

        let ranked = dimension_report(
            &conn,
            &["a".into()],
            "pageview",
            &ranges,
            &Dimension::Path,
            None,
            &[],
            &Metric::AvgTimeOnSite,
            1,
            2,
        )
        .expect("failed to build ranked average time report")
        .remove(0);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked["/"], Some(90.0));
    }

    #[test]
    fn dimension_report_lists_property_keys_and_values() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let conn = app.events_conn().unwrap();
        let now = Utc::now();
        for (visitor, event, properties, path) in [
            ("one", "pageview", Some("{'plan'='pro', 'ref'='x'}"), "/"),
            ("two", "pageview", Some("{'plan'='pro'}"), "/pricing"),
            ("three", "pageview", Some("{'plan'='free'}"), "/"),
            ("four", "pageview", None, "/"),
            ("one", "signup", Some("{'source'='ad'}"), "/"),
        ] {
            conn.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at, path, properties) values ('a', ?, ?, ?, ?, ?::map(varchar, varchar))",
                duckdb::params![visitor, event, now, path, properties],
            )
            .unwrap();
        }
        let range = DateRange { start: now - Duration::hours(1), end: now + Duration::hours(1) };
        let ranges = [range];
        let report = |event: &str, key: Option<&str>, filters: &[DimensionFilter]| {
            dimension_report(
                &conn,
                &["a".into()],
                event,
                &ranges,
                &Dimension::Property,
                key,
                filters,
                &Metric::Views,
                10,
                0,
            )
            .unwrap()
            .remove(0)
            .into_iter()
            .collect::<Vec<_>>()
        };
        let row = |name: &str, value: f64| (name.to_string(), Some(value));

        assert_eq!(report("pageview", None, &[]), [row("plan", 3.0), row("ref", 1.0)]);
        assert_eq!(report("signup", None, &[]), [row("source", 1.0)]);
        assert_eq!(report("pageview", Some("plan"), &[]), [row("free", 1.0), row("pro", 2.0)]);
        assert!(report("pageview", Some("missing"), &[]).is_empty());

        let filter = |key: &str, filter_type: FilterType, value: Option<&str>| DimensionFilter {
            dimension: Dimension::Property,
            key: Some(key.into()),
            filter_type,
            inversed: None,
            strict: None,
            value: value.map(Into::into),
        };
        let pro = filter("plan", FilterType::Equal, Some("pro"));
        assert_eq!(report("pageview", Some("ref"), std::slice::from_ref(&pro)), [row("x", 1.0)]);
        let unset = filter("plan", FilterType::IsNull, None);
        assert_eq!(
            dimension_report(
                &conn,
                &["a".into()],
                "pageview",
                &ranges,
                &Dimension::Path,
                None,
                &[unset],
                &Metric::Views,
                10,
                0,
            )
            .unwrap()
            .remove(0)
            .into_iter()
            .collect::<Vec<_>>(),
            [row("/", 1.0)]
        );
    }
}
