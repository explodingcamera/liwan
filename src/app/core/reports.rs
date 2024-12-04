use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

use crate::app::DuckDBConn;
use crate::utils::duckdb::{repeat_vars, ParamVec};
use duckdb::params_from_iter;
use eyre::{bail, Result};
use poem_openapi::{Enum, Object};

#[derive(Object, Debug, Clone)]
pub struct DateRange {
    pub start: time::OffsetDateTime,
    pub end: time::OffsetDateTime,
}

impl DateRange {
    pub fn prev(&self) -> DateRange {
        let duration = self.end - self.start;
        DateRange { start: self.start - duration, end: self.start }
    }
}

impl Display for DateRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}

#[derive(Debug, Enum, Clone, Copy)]
#[oai(rename_all = "snake_case")]
pub enum Metric {
    Views,
    UniqueVisitors,
    BounceRate,
    AvgTimeOnSite,
}

#[derive(Debug, Enum, Clone, Copy, PartialEq)]
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
    UtmSource,
    UtmMedium,
    UtmCampaign,
    UtmContent,
    UtmTerm,
}

#[derive(Enum, Debug, Clone, Copy, PartialEq)]
#[oai(rename_all = "snake_case")]
pub enum FilterType {
    // Generic filters
    IsNull,

    // String filters
    Equal,
    Contains,
    StartsWith,
    EndsWith,

    // Boolean filters
    IsTrue,
    IsFalse,
}

pub type ReportGraph = Vec<f64>;
pub type ReportTable = BTreeMap<String, f64>;

#[derive(Object, Clone, Debug, Default)]
#[oai(rename_all = "camelCase")]
pub struct ReportStats {
    pub total_views: u64,
    pub unique_visitors: u64,
    pub bounce_rate: f64,
    pub avg_time_on_site: f64,
}

#[derive(Object, Debug, Clone)]
#[oai(rename_all = "camelCase")]
pub struct DimensionFilter {
    /// The dimension to filter by
    dimension: Dimension,

    /// The type of filter to apply
    /// Note that some filters may not be applicable to all dimensions
    filter_type: FilterType,

    /// Whether to invert the filter (e.g. not equal, not contains)
    /// Defaults to false
    inversed: Option<bool>,

    /// Whether to filter by the strict value (case-sensitive, exact match)
    strict: Option<bool>,

    /// The value to filter by
    /// For `FilterType::IsNull` this should be `None`
    value: Option<String>,
}

fn filter_sql(filters: &[DimensionFilter]) -> Result<(String, ParamVec)> {
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
                Dimension::Path => format!("path {filter_value}"),
                Dimension::Fqdn => format!("fqdn {filter_value}"),
                Dimension::Referrer => format!("referrer {filter_value}"),
                Dimension::Platform => format!("platform {filter_value}"),
                Dimension::Browser => format!("browser {filter_value}"),
                Dimension::Mobile => format!("mobile::text {filter_value}"),
                Dimension::Country => format!("country {filter_value}"),
                Dimension::City => format!("city {filter_value}"),
                Dimension::UtmSource => format!("utm_source {filter_value}"),
                Dimension::UtmMedium => format!("utm_medium {filter_value}"),
                Dimension::UtmCampaign => format!("utm_campaign {filter_value}"),
                Dimension::UtmContent => format!("utm_content {filter_value}"),
                Dimension::UtmTerm => format!("utm_term {filter_value}"),
            })
        })
        .collect::<Result<Vec<String>>>()?;

    Ok((format!("and ({})", filter_clauses.join(" and ")), params))
}

fn metric_sql(metric: Metric) -> String {
    match metric {
        Metric::Views => "count(sd.created_at)",
        Metric::UniqueVisitors => {
            // Count the number of unique visitors as the number of distinct visitor IDs
            "--sql
            count(distinct sd.visitor_id)"
        }
        Metric::BounceRate => {
            // total sessions: no time_to_next_event / time_to_next_event is null
            // bounce sessions: time to next / time to prev are both null or both > interval '30 minutes'
            "--sql
            coalesce(
                count(distinct sd.visitor_id)
                    filter (where (sd.time_to_next_event is null or sd.time_to_next_event > interval '30 minutes') and
                                 (sd.time_from_last_event is null or sd.time_from_last_event > interval '30 minutes')) /
                nullif(count(distinct sd.visitor_id) filter (where sd.time_to_next_event is null or sd.time_to_next_event > interval '30 minutes'), 0),
                1
            )
            "
        }
        Metric::AvgTimeOnSite => {
            // avg time_to_next_event where time_to_next_event <= 1800 and time_to_next_event is not null
            "--sql
            coalesce(avg(extract(epoch from sd.time_to_next_event)) filter (where sd.time_to_next_event is not null and sd.time_to_next_event <= interval '30 minutes'), 0)"
        }
    }
    .to_owned()
}

pub fn earliest_timestamp(conn: &DuckDBConn, entities: &[String]) -> Result<Option<time::OffsetDateTime>> {
    if entities.is_empty() {
        return Ok(None);
    }

    let vars = repeat_vars(entities.len());
    let query = format!(
        "--sql
            select min(created_at) from events
            where entity_id in ({vars});
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let rows = stmt.query_map(params_from_iter(entities), |row| row.get(0))?;
    let earliest_timestamp = rows.collect::<Result<Vec<Option<time::OffsetDateTime>>, duckdb::Error>>()?;
    Ok(earliest_timestamp[0])
}

pub fn online_users(conn: &DuckDBConn, entities: &[String]) -> Result<u64> {
    if entities.is_empty() {
        return Ok(0);
    }

    let vars = repeat_vars(entities.len());
    let query = format!(
        "--sql
            select count(distinct visitor_id) from events
            where
                entity_id in ({vars}) and
                created_at >= (now()::timestamp - (interval 5 minute));
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let rows = stmt.query_map(params_from_iter(entities), |row| row.get(0))?;
    let online_users = rows.collect::<Result<Vec<u64>, duckdb::Error>>()?;
    Ok(online_users[0])
}

pub fn overall_report(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    data_points: u32,
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportGraph> {
    if entities.is_empty() {
        return Ok(vec![0.; data_points as usize]);
    }

    let mut params = ParamVec::new();

    let (filters_sql, filters_params) = filter_sql(filters)?;
    let metric_sql = metric_sql(*metric);

    let entity_vars = repeat_vars(entities.len());

    params.push(range.start);
    params.push(range.end);
    params.push(data_points);
    params.push(data_points);
    params.push(event);
    params.extend(entities);
    params.extend_from_params(filters_params);
    params.push(range.end);

    let query = format!(
        "--sql
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
                    time_from_last_event,
                    time_to_next_event,
                from events, params
                where
                    event = ?::text and
                    created_at between params.start_time and params.end_time and
                    entity_id in ({entity_vars})
                    {filters_sql}
            ),
            event_bins as (
                select
                    bin_start,
                    {metric_sql} as metric_value
                from
                    time_bins tb
                    left join session_data sd
                    on sd.created_at >= tb.bin_start and sd.created_at < coalesce(tb.bin_end, ?::timestamp)
                group by
                    bin_start
            )
        select
            tb.bin_start,
            coalesce(eb.metric_value, 0)
        from
            time_bins tb
            left join event_bins eb on tb.bin_start = eb.bin_start
        order by
            tb.bin_start;
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;

    match metric {
        Metric::Views | Metric::UniqueVisitors => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get(1))?;
            let report_graph = rows.collect::<Result<Vec<f64>, duckdb::Error>>()?;
            Ok(report_graph)
        }
        Metric::AvgTimeOnSite | Metric::BounceRate => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| row.get::<_, Option<f64>>(1))?;
            let report_graph =
                rows.map(|r| r.map(|v| v.unwrap_or(0.0))).collect::<Result<Vec<f64>, duckdb::Error>>()?;
            Ok(report_graph)
        }
    }
}

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
    let (filters_sql, filters_params) = filter_sql(filters)?;

    let metric_total = metric_sql(Metric::Views);
    let metric_unique_visitors = metric_sql(Metric::UniqueVisitors);
    let metric_bounce_rate = metric_sql(Metric::BounceRate);
    let metric_avg_time_on_site = metric_sql(Metric::AvgTimeOnSite);

    let mut params = ParamVec::new();
    params.push(range.start);
    params.push(range.end);
    params.push(event);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let query = format!(
        "--sql
        with
            params as (
                select
                    ?::timestamp as start_time,
                    ?::timestamp as end_time,
            ),
            session_data as (
                select
                    visitor_id,
                    created_at,
                    time_from_last_event,
                    time_to_next_event,
                from events, params
                where
                    event = ?::text and
                    created_at between params.start_time and params.end_time and
                    entity_id in ({entity_vars})
                    {filters_sql}
            )
        select
            {metric_total} as total_views,
            {metric_unique_visitors} as unique_visitors,
            {metric_bounce_rate} as bounce_rate,
            {metric_avg_time_on_site} as avg_time_on_site
        from
            session_data sd;
    "
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let result = stmt.query_row(duckdb::params_from_iter(params), |row| {
        Ok(ReportStats {
            total_views: row.get(0)?,
            unique_visitors: row.get(1)?,
            bounce_rate: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            avg_time_on_site: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
        })
    })?;

    Ok(result)
}

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
    let (filters_sql, filters_params) = filter_sql(filters)?;

    let metric_column = metric_sql(*metric);
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
        Dimension::UtmSource => ("utm_source", "utm_source"),
        Dimension::UtmMedium => ("utm_medium", "utm_medium"),
        Dimension::UtmCampaign => ("utm_campaign", "utm_campaign"),
        Dimension::UtmContent => ("utm_content", "utm_content"),
        Dimension::UtmTerm => ("utm_term", "utm_term"),
    };

    params.push(range.start);
    params.push(range.end);
    params.push(event);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let query = format!(
        "--sql
        with
            params as (
                select
                    ?::timestamp as start_time,
                    ?::timestamp as end_time,
            ),
            session_data as (
                select
                    coalesce({dimension_column}, 'Unknown') as dimension_value,
                    visitor_id,
                    created_at,
                    time_from_last_event,
                    time_to_next_event,
                from events sd, params
                where
                    event = ?::text and
                    created_at between params.start_time and params.end_time and
                    entity_id in ({entity_vars})
                    {filters_sql}
                group by
                    {group_by_columns}, visitor_id, created_at, time_from_last_event, time_to_next_event
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
