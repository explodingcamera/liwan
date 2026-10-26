use crate::utils::duckdb::ParamVec;
use anyhow::{Result, bail};

use super::{Dimension, DimensionFilter, FilterType, Metric};

pub(super) const SESSION_DURATION_SQL: &str = "interval '30 minutes'";

pub(super) fn measured_duration_sql(alias: &str) -> String {
    format!(
        "extract(epoch from case
            when {alias}.time_to_next_event between interval '0 seconds' and {SESSION_DURATION_SQL}
                then {alias}.time_to_next_event
            when {alias}.exited_at between {alias}.created_at and {alias}.created_at + {SESSION_DURATION_SQL}
                then {alias}.exited_at - {alias}.created_at
        end)"
    )
}

pub(super) fn metric_sample_count_sql(metric: Metric, alias: &str) -> String {
    match metric {
        Metric::BounceRate => format!(
            "count(*) filter (where {alias}.time_from_last_event is null or {alias}.time_from_last_event > {SESSION_DURATION_SQL})"
        ),
        Metric::AvgTimeOnSite => format!("count({})", measured_duration_sql(alias)),
        _ => "null::ubigint".to_string(),
    }
}

pub(super) fn build_filter_clause(filters: &[DimensionFilter]) -> Result<(String, ParamVec<'_>)> {
    let mut params = ParamVec::new();

    if filters.is_empty() {
        return Ok((String::new(), params));
    }

    let filter_clauses = filters
		.iter()
		.map(|filter| {
			if filter.dimension == Dimension::Property {
				params.push(filter.key.clone());
			}

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

					if inversed {
						format!("not {sql}")
					} else {
						sql.to_owned()
					}
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
				Dimension::Property if filter.filter_type == FilterType::IsNull => format!(
					"map_contains(properties, ?) is {}true",
					if filter.inversed.unwrap_or(false) { "" } else { "not " }
				),
				Dimension::Property => format!("map_extract_value(properties, ?) {filter_value}"),
			})
		})
		.collect::<Result<Vec<String>>>()?;

    Ok((format!("and ({})", filter_clauses.join(" and ")), params))
}

pub(super) fn metric_aggregate_sql(metric: Metric, alias: &str) -> String {
    match metric {
        Metric::Views => format!("count({alias}.created_at)"),
        Metric::UniqueVisitors => format!("count(distinct {alias}.visitor_group_id)"),
        Metric::BounceRate => format!(
            "coalesce(
                avg(({alias}.time_to_next_event is null or {alias}.time_to_next_event > {SESSION_DURATION_SQL})::integer)
                    filter (where {alias}.time_from_last_event is null or {alias}.time_from_last_event > {SESSION_DURATION_SQL}),
                1
            )"
        ),
        Metric::AvgTimeOnSite => format!("coalesce(avg({}), 0)", measured_duration_sql(alias)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounce_rate_counts_only_session_entries() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "create table events as
             select name, previous_seconds * interval '1 second' as time_from_last_event,
                 next_seconds * interval '1 second' as time_to_next_event
             from (values
                 ('mixed', null, null), ('mixed', 1801, 1800), ('mixed', 1801, 1801),
                 ('mixed', 1800, null), ('mixed', 0, null),
                 ('no entries', 0, null), ('no entries', 1800, 1801),
                 ('not bounced', null, 0)
             ) durations(name, previous_seconds, next_seconds)",
        )
        .unwrap();
        let aggregate = metric_aggregate_sql(Metric::BounceRate, "sd");
        let samples = metric_sample_count_sql(Metric::BounceRate, "sd");
        let query = format!("select name, {aggregate}, {samples} from events sd group by name order by name");
        let rows = conn
            .prepare(&query)
            .unwrap()
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, u64>(2)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![("mixed".into(), 2.0 / 3.0, 3), ("no entries".into(), 1.0, 0), ("not bounced".into(), 0.0, 1)]
        );

        let empty = conn
            .query_row(&format!("select {aggregate}, {samples} from events sd where false"), [], |row| {
                Ok((row.get::<_, f64>(0)?, row.get::<_, u64>(1)?))
            })
            .unwrap();
        assert_eq!(empty, (1.0, 0));
    }

    #[test]
    fn property_presence_handles_null_and_empty_maps() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        for (inversed, expected) in [(false, vec![0, 1, 2]), (true, vec![3, 4])] {
            let filters = [DimensionFilter {
                dimension: Dimension::Property,
                key: Some("plan".into()),
                filter_type: FilterType::IsNull,
                inversed: Some(inversed),
                strict: None,
                value: None,
            }];
            let (clause, params) = build_filter_clause(&filters).unwrap();
            let query = format!(
                "select id from (values
                    (0, null::map(varchar, varchar)),
                    (1, map()),
                    (2, map(['other'], ['value'])),
                    (3, map(['plan'], ['pro'])),
                    (4, map(['plan'], ['']))
                 ) events(id, properties) where true {clause} order by id"
            );
            let values = conn
                .prepare(&query)
                .unwrap()
                .query_map(duckdb::params_from_iter(params), |row| row.get::<_, i32>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(values, expected);
        }
    }

    #[test]
    fn measured_duration_handles_boundaries_and_fallbacks() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        let query = format!(
            "select {} from (
                select '2026-01-01'::timestamp as created_at,
                    next_seconds * interval '1 second' as time_to_next_event,
                    '2026-01-01'::timestamp + exit_seconds * interval '1 second' as exited_at
                from (values
                    (0, null), (1800, null), (60, 120),
                    (null, 90), (-1, 90), (1801, 90),
                    (null, 0), (null, 1800),
                    (null, null), (null, -1), (null, 1801)
                ) durations(next_seconds, exit_seconds)
            ) sd",
            measured_duration_sql("sd")
        );
        let values = conn
            .prepare(&query)
            .unwrap()
            .query_map([], |row| row.get::<_, Option<f64>>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            values,
            vec![
                Some(0.0),
                Some(1800.0),
                Some(60.0),
                Some(90.0),
                Some(90.0),
                Some(90.0),
                Some(0.0),
                Some(1800.0),
                None,
                None,
                None
            ]
        );
    }
}
