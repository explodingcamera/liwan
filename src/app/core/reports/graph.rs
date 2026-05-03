use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use anyhow::{Context, Result};
use chrono::{DateTime, Days, Duration, LocalResult, NaiveDate, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

use super::shared::{build_filter_clause, metric_aggregate_sql};
use super::{DateRange, DimensionFilter, GraphInterval, Metric, ReportGraph, ReportGraphPoint};

fn zero_report_graph(buckets: &[DateRange]) -> ReportGraph {
    buckets.iter().map(|bucket| ReportGraphPoint { bin_start: bucket.start, value: 0.0 }).collect()
}

fn build_time_bins_values_sql(bucket_count: usize) -> String {
    (0..bucket_count).map(|idx| format!("({idx}, ?::timestamp, ?::timestamp)")).collect::<Vec<_>>().join(", ")
}

fn resolve_local_day_start(timezone: Tz, date: NaiveDate) -> Result<DateTime<Utc>> {
    let local_midnight = date.and_hms_opt(0, 0, 0).context("Failed to build local midnight")?;

    let resolved = match timezone.from_local_datetime(&local_midnight) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(earlier, later) => earlier.min(later),
        LocalResult::None => {
            let mut candidate = local_midnight;
            let mut resolved = None;

            for _ in 0..180 {
                candidate += Duration::minutes(1);
                match timezone.from_local_datetime(&candidate) {
                    LocalResult::Single(dt) => {
                        resolved = Some(dt);
                        break;
                    }
                    LocalResult::Ambiguous(earlier, later) => {
                        resolved = Some(earlier.min(later));
                        break;
                    }
                    LocalResult::None => continue,
                }
            }

            resolved.with_context(|| format!("Failed to resolve local day start for {date} in {timezone}"))?
        }
    };

    Ok(resolved.with_timezone(&Utc))
}

pub fn build_graph_buckets(
    range: &DateRange,
    interval: GraphInterval,
    timezone: Option<&str>,
) -> Result<Vec<DateRange>> {
    if range.start >= range.end {
        return Ok(Vec::new());
    }

    let timezone_name = timezone.unwrap_or("UTC");
    let timezone: Tz = timezone_name.parse().with_context(|| format!("Invalid timezone: {timezone_name}"))?;

    let aligned_start = match interval {
        GraphInterval::Hour => {
            let start_local = range.start.with_timezone(&timezone);
            range.start
                - Duration::minutes(i64::from(start_local.minute()))
                - Duration::seconds(i64::from(start_local.second()))
                - Duration::nanoseconds(i64::from(start_local.nanosecond()))
        }
        GraphInterval::Day => resolve_local_day_start(timezone, range.start.with_timezone(&timezone).date_naive())?,
    };

    let mut buckets = Vec::new();
    let mut bucket_start = aligned_start;

    while bucket_start < range.end {
        let next_bucket_start = match interval {
            GraphInterval::Hour => bucket_start + Duration::hours(1),
            GraphInterval::Day => {
                let next_date = bucket_start
                    .with_timezone(&timezone)
                    .date_naive()
                    .checked_add_days(Days::new(1))
                    .context("Failed to advance bucket date")?;
                resolve_local_day_start(timezone, next_date)?
            }
        };

        buckets.push(DateRange {
            start: bucket_start,
            end: if next_bucket_start < range.end { next_bucket_start } else { range.end },
        });
        bucket_start = next_bucket_start;
    }

    Ok(buckets)
}

pub fn overall_report(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    range: &DateRange,
    buckets: &[DateRange],
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<ReportGraph> {
    if buckets.is_empty() {
        return Ok(Vec::new());
    }

    if entities.is_empty() {
        return Ok(zero_report_graph(buckets));
    }

    let mut params = ParamVec::new();

    let (filters_sql, filters_params) = build_filter_clause(filters)?;
    let metric_sql = metric_aggregate_sql(*metric, "sd");
    let time_bins_sql = build_time_bins_values_sql(buckets.len());

    let entity_vars = repeat_vars(entities.len());

    for bucket in buckets {
        params.push(bucket.start);
        params.push(bucket.end);
    }
    params.push(event);
    params.push(range.start);
    params.push(range.end);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let query = format!(
        "--sql
		with
			time_bins(bucket_idx, bin_start, bin_end) as (
				values {time_bins_sql}
			),
			session_data as (
				select
					e.visitor_id,
					e.created_at,
					e.time_from_last_event,
					e.time_to_next_event
				from events e
				where
					e.event = ?::text and
					e.created_at >= ?::timestamp and e.created_at < ?::timestamp and
					e.entity_id in ({entity_vars})
					{filters_sql}
			),
			bucketed_events as (
				select
					tb.bucket_idx,
					sd.visitor_id,
					sd.created_at,
					sd.time_from_last_event,
					sd.time_to_next_event
				from (select * from session_data order by created_at) sd
				asof join (select * from time_bins order by bin_start) tb
					on sd.created_at >= tb.bin_start
				where sd.created_at < tb.bin_end
			),
			event_bins as (
				select
					sd.bucket_idx,
					{metric_sql} as metric_value
				from bucketed_events sd
				group by sd.bucket_idx
			)
		select
			tb.bin_start,
			coalesce(eb.metric_value, 0)
		from time_bins tb
		left join event_bins eb on tb.bucket_idx = eb.bucket_idx
		order by tb.bucket_idx;
	"
    );

    let mut stmt = conn.prepare_cached(&query)?;

    match metric {
        Metric::Views | Metric::UniqueVisitors => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| {
                Ok(ReportGraphPoint { bin_start: row.get(0)?, value: row.get(1)? })
            })?;
            let report_graph = rows.collect::<Result<Vec<ReportGraphPoint>, duckdb::Error>>()?;
            Ok(report_graph)
        }
        Metric::AvgTimeOnSite | Metric::BounceRate => {
            let rows = stmt.query_map(duckdb::params_from_iter(params), |row| {
                Ok(ReportGraphPoint { bin_start: row.get(0)?, value: row.get::<_, Option<f64>>(1)?.unwrap_or(0.0) })
            })?;
            let report_graph = rows.collect::<Result<Vec<ReportGraphPoint>, duckdb::Error>>()?;
            Ok(report_graph)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Liwan, models::Event};
    use crate::config::Config;

    fn local_datetime(timezone: Tz, year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        timezone
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("failed to construct local datetime")
            .with_timezone(&Utc)
    }

    fn test_event(created_at: DateTime<Utc>) -> Event {
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: format!("visitor-{created_at}"),
            event: "pageview".to_string(),
            created_at,
            fqdn: Some("example.com".to_string()),
            path: Some("/".to_string()),
            referrer: None,
            platform: None,
            browser: None,
            mobile: None,
            country: None,
            city: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            utm_content: None,
            utm_term: None,
            screen_width: None,
            orientation: None,
        }
    }

    #[test]
    fn build_graph_buckets_aligns_hours_to_local_boundaries() {
        let timezone = "Asia/Kolkata";
        let range = DateRange {
            start: local_datetime(Tz::Asia__Kolkata, 2024, 1, 1, 23, 23),
            end: local_datetime(Tz::Asia__Kolkata, 2024, 1, 2, 1, 23),
        };

        let buckets =
            build_graph_buckets(&range, GraphInterval::Hour, Some(timezone)).expect("failed to build buckets");
        let local_starts = buckets
            .iter()
            .map(|bucket| bucket.start.with_timezone(&Tz::Asia__Kolkata).format("%Y-%m-%d %H:%M").to_string())
            .collect::<Vec<_>>();

        assert_eq!(local_starts, vec!["2024-01-01 23:00", "2024-01-02 00:00", "2024-01-02 01:00"]);
        assert_eq!(buckets.last().expect("missing last bucket").end, range.end);
    }

    #[test]
    fn build_graph_buckets_aligns_days_to_local_midnight() {
        let timezone = "America/New_York";
        let range = DateRange {
            start: local_datetime(Tz::America__New_York, 2024, 1, 1, 0, 0),
            end: local_datetime(Tz::America__New_York, 2024, 1, 3, 12, 0),
        };

        let buckets = build_graph_buckets(&range, GraphInterval::Day, Some(timezone)).expect("failed to build buckets");
        let local_starts = buckets
            .iter()
            .map(|bucket| bucket.start.with_timezone(&Tz::America__New_York).format("%Y-%m-%d %H:%M").to_string())
            .collect::<Vec<_>>();

        assert_eq!(local_starts, vec!["2024-01-01 00:00", "2024-01-02 00:00", "2024-01-03 00:00"]);
        assert_eq!(
            buckets
                .last()
                .expect("missing last bucket")
                .end
                .with_timezone(&Tz::America__New_York)
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            "2024-01-03 12:00"
        );
    }

    #[test]
    fn overall_report_includes_start_and_excludes_end() {
        let app = Liwan::new_memory(Config::default()).expect("failed to create app");
        app.seed_database(0).expect("failed to seed app");

        let start = local_datetime(Tz::UTC, 2024, 1, 1, 0, 0);
        let middle = local_datetime(Tz::UTC, 2024, 1, 2, 12, 0);
        let end = local_datetime(Tz::UTC, 2024, 1, 3, 0, 0);

        app.events
            .append(vec![test_event(start), test_event(middle), test_event(end)].into_iter())
            .expect("failed to append events");

        let range = DateRange { start, end };
        let buckets = build_graph_buckets(&range, GraphInterval::Day, Some("UTC")).expect("failed to build buckets");
        let conn = app.events_conn().expect("failed to get events conn");
        let report =
            overall_report(&conn, &["entity-1".to_string()], "pageview", &range, &buckets, &[], &Metric::Views)
                .expect("failed to build report");

        let values = report.iter().map(|point| point.value).collect::<Vec<_>>();
        assert_eq!(values, vec![1.0, 1.0]);
    }
}
