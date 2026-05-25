use crate::utils::duckdb::ParamVec;
use anyhow::{Result, bail};

use super::{Dimension, DimensionFilter, FilterType, Metric};

pub(super) const SESSION_DURATION_SQL: &str = "interval '30 minutes'";

pub(super) fn build_filter_clause(filters: &[DimensionFilter]) -> Result<(String, ParamVec<'_>)> {
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
			})
		})
		.collect::<Result<Vec<String>>>()?;

    Ok((format!("and ({})", filter_clauses.join(" and ")), params))
}

pub(super) fn metric_aggregate_sql(metric: Metric, alias: &str) -> String {
    match metric {
        Metric::Views => format!("count({alias}.created_at)"),
        Metric::UniqueVisitors => format!("count(distinct {alias}.visitor_group_id)"),
        Metric::BounceRate => {
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
            format!(
				"--sql
			coalesce(avg(extract(epoch from {alias}.time_to_next_event)) filter (where {alias}.time_to_next_event is not null and {alias}.time_to_next_event <= {SESSION_DURATION_SQL}), 0)"
			)
        }
    }
}
