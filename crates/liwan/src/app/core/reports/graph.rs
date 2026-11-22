use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use crate::utils::timestamp::SqlTimestamp;
use anyhow::{Context, Result};
use jiff::{RoundMode, SignedDuration, ToSpan, Unit, ZonedDateTimeRound, tz::TimeZone};

use super::shared::{build_filter_clause, metric_aggregate_sql, range_events_sql};
use super::{DateRange, DimensionFilter, GraphInterval, Metric, ReportGraph, ReportGraphPoint};

fn zero_report_graph(buckets: &[DateRange]) -> ReportGraph {
    buckets.iter().map(|bucket| ReportGraphPoint { bin_start: bucket.start, value: 0.0 }).collect()
}

fn build_time_bins_values_sql(buckets: &[Vec<DateRange>]) -> String {
    buckets
        .iter()
        .enumerate()
        .flat_map(|(range_idx, buckets)| {
            (0..buckets.len()).map(move |bucket_idx| format!("({range_idx}, {bucket_idx}, ?::timestamp, ?::timestamp)"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Split a date range into graph buckets aligned to the selected timezone
pub fn build_graph_buckets(
    range: &DateRange,
    interval: GraphInterval,
    timezone: Option<&str>,
    max_datapoints: usize,
) -> Result<Vec<DateRange>> {
    if range.start >= range.end {
        return Ok(Vec::new());
    }

    let timezone_name = timezone.unwrap_or("UTC");
    let timezone = TimeZone::get(timezone_name).with_context(|| format!("Invalid timezone: {timezone_name}"))?;
    let date = timezone.to_datetime(range.start).date();
    let mut bucket_date = match interval {
        GraphInterval::Hour | GraphInterval::Day => date,
        GraphInterval::Week => date.checked_sub(date.weekday().to_monday_zero_offset().days())?,
        GraphInterval::Month => date.first_of_month(),
    };
    let mut bucket_start = match interval {
        GraphInterval::Hour => range
            .start
            .to_zoned_datetime(timezone.clone())
            .round(ZonedDateTimeRound::new().smallest(Unit::Hour).mode(RoundMode::Floor))?
            .timestamp(),
        _ => bucket_date.to_zoned_datetime(timezone.clone())?.timestamp(),
    };

    let calendar_step = match interval {
        GraphInterval::Hour => None,
        GraphInterval::Day => Some(1.day()),
        GraphInterval::Week => Some(1.week()),
        GraphInterval::Month => Some(1.month()),
    };
    let mut buckets = Vec::new();

    while bucket_start < range.end {
        if buckets.len() >= max_datapoints {
            anyhow::bail!("Too many data points");
        }
        let next_bucket_start = if let Some(step) = calendar_step {
            bucket_date = bucket_date.checked_add(step)?;
            bucket_date.to_zoned_datetime(timezone.clone())?.timestamp()
        } else {
            bucket_start.checked_add(SignedDuration::from_hours(1)).context("Graph end is out of bounds")?
        };

        // A timezone transition can skip a whole calendar date.
        if next_bucket_start > bucket_start {
            buckets.push(DateRange { start: bucket_start, end: next_bucket_start.min(range.end) });
        }
        bucket_start = next_bucket_start;
    }

    Ok(buckets)
}

/// Build graph reports for all ranges in one query using each range's precomputed buckets.
pub fn overall_report(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    ranges: &[DateRange],
    buckets: &[Vec<DateRange>],
    filters: &[DimensionFilter],
    metric: &Metric,
) -> Result<Vec<ReportGraph>> {
    anyhow::ensure!(ranges.len() == buckets.len(), "Each graph range must have its own buckets");
    if ranges.is_empty() {
        return Ok(Vec::new());
    }

    if entities.is_empty() || buckets.iter().all(Vec::is_empty) {
        return Ok(buckets.iter().map(|buckets| zero_report_graph(buckets)).collect());
    }

    let mut params = ParamVec::new();
    let range_events = range_events_sql(ranges, &mut params);

    let (filters_sql, filters_params) = build_filter_clause(filters)?;
    // Apply the same filters, but not the minimum sample counts used by summary and dimension reports.
    // Suppressing sparse time buckets would create misleading gaps in the graph.
    let metric_sql = metric_aggregate_sql(*metric, "sd");
    let time_bins_sql = build_time_bins_values_sql(buckets);

    let entity_vars = repeat_vars(entities.len());

    for bucket in buckets.iter().flatten() {
        params.push(SqlTimestamp(bucket.start));
        params.push(SqlTimestamp(bucket.end));
    }
    params.push(event);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let query = format!(
        "--sql
		with
			range_events as ({range_events}),
			time_bins(range_idx, bucket_idx, bin_start, bin_end) as (
				values {time_bins_sql}
			),
			session_data as (
				select
					e.range_idx,
					e.visitor_group_id,
					e.created_at,
					e.time_from_last_event,
					e.time_to_next_event,
					e.exited_at
				from range_events e
				where
					e.event = ?::text and
					e.entity_id in ({entity_vars})
					{filters_sql}
			),
			event_bins as (
				select
					tb.range_idx,
					tb.bucket_idx,
					{metric_sql} as metric_value
				from session_data sd
				asof join time_bins tb
					on sd.range_idx = tb.range_idx and sd.created_at >= tb.bin_start
				where sd.created_at < tb.bin_end
				group by tb.range_idx, tb.bucket_idx
			)
		select
			tb.range_idx,
			tb.bin_start,
			coalesce(eb.metric_value, 0)
		from time_bins tb
		left join event_bins eb on tb.range_idx = eb.range_idx and tb.bucket_idx = eb.bucket_idx
		order by tb.range_idx, tb.bucket_idx;
	"
    );

    let mut stmt = conn.prepare_cached(&query)?;

    let mut rows = stmt.query(duckdb::params_from_iter(params))?;
    let mut reports = vec![Vec::new(); ranges.len()];
    while let Some(row) = rows.next()? {
        reports[row.get::<_, usize>(0)?]
            .push(ReportGraphPoint { bin_start: row.get::<_, SqlTimestamp>(1)?.0, value: row.get(2)? });
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Liwan, models::Event};
    use crate::config::Config;
    use jiff::Timestamp;

    fn local_datetime(timezone: &str, year: i16, month: i8, day: i8, hour: i8, minute: i8) -> Timestamp {
        jiff::civil::date(year, month, day)
            .at(hour, minute, 0, 0)
            .to_zoned_datetime(TimeZone::get(timezone).unwrap())
            .unwrap()
            .timestamp()
    }

    fn test_event(created_at: Timestamp) -> Event {
        Event {
            entity_id: "entity-1".to_string(),
            visitor_group_id: format!("visitor-{created_at}"),
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
            properties: Default::default(),
            track_sessions: true,
        }
    }

    #[test]
    fn build_graph_buckets_aligns_hours_to_local_boundaries() {
        let timezone = "Asia/Kolkata";
        let range = DateRange {
            start: local_datetime("Asia/Kolkata", 2024, 1, 1, 23, 23),
            end: local_datetime("Asia/Kolkata", 2024, 1, 2, 1, 23),
        };

        let buckets =
            build_graph_buckets(&range, GraphInterval::Hour, Some(timezone), 2000).expect("failed to build buckets");
        let local_starts = buckets
            .iter()
            .map(|bucket| {
                bucket
                    .start
                    .to_zoned_datetime(TimeZone::get("Asia/Kolkata").unwrap())
                    .strftime("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(local_starts, vec!["2024-01-01 23:00", "2024-01-02 00:00", "2024-01-02 01:00"]);
        assert_eq!(buckets.last().expect("missing last bucket").end, range.end);
    }

    #[test]
    fn hourly_buckets_preserve_both_repeated_hours() {
        let range =
            DateRange { start: "2024-11-03T05:30:00Z".parse().unwrap(), end: "2024-11-03T06:30:00Z".parse().unwrap() };
        let buckets = build_graph_buckets(&range, GraphInterval::Hour, Some("America/New_York"), 2).unwrap();
        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0].start, "2024-11-03T05:00:00Z".parse().unwrap());
        assert_eq!(buckets[1].start, "2024-11-03T06:00:00Z".parse().unwrap());
        let second_hour = DateRange { start: buckets[1].start + SignedDuration::from_mins(15), end: range.end };
        assert_eq!(
            build_graph_buckets(&second_hour, GraphInterval::Hour, Some("America/New_York"), 1).unwrap(),
            [buckets[1].clone()],
        );
    }

    #[test]
    fn build_graph_buckets_aligns_days_to_local_midnight() {
        let timezone = "America/New_York";
        let range = DateRange {
            start: local_datetime("America/New_York", 2024, 1, 1, 0, 0),
            end: local_datetime("America/New_York", 2024, 1, 3, 12, 0),
        };

        let buckets =
            build_graph_buckets(&range, GraphInterval::Day, Some(timezone), 2000).expect("failed to build buckets");
        let local_starts = buckets
            .iter()
            .map(|bucket| {
                bucket
                    .start
                    .to_zoned_datetime(TimeZone::get("America/New_York").unwrap())
                    .strftime("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(local_starts, vec!["2024-01-01 00:00", "2024-01-02 00:00", "2024-01-03 00:00"]);
        assert_eq!(
            buckets
                .last()
                .expect("missing last bucket")
                .end
                .to_zoned_datetime(TimeZone::get("America/New_York").unwrap())
                .strftime("%Y-%m-%d %H:%M")
                .to_string(),
            "2024-01-03 12:00"
        );
    }

    #[test]
    fn daily_buckets_follow_daylight_saving() {
        let timezone = "America/New_York";
        for (month, day, hours) in [(3, 10, 23), (11, 3, 25)] {
            let range = DateRange {
                start: local_datetime(timezone, 2024, month, day, 0, 0),
                end: local_datetime(timezone, 2024, month, day + 1, 0, 0),
            };
            let buckets = build_graph_buckets(&range, GraphInterval::Day, Some(timezone), 2).unwrap();
            assert_eq!(buckets, [range]);
            assert_eq!(buckets[0].duration().as_hours(), hours);
        }
    }

    #[test]
    fn build_graph_buckets_aligns_weeks_and_months() {
        let timezone = "America/New_York";
        let range = DateRange {
            start: local_datetime("America/New_York", 2024, 2, 28, 12, 0),
            end: local_datetime("America/New_York", 2024, 4, 2, 12, 0),
        };

        let weeks = build_graph_buckets(&range, GraphInterval::Week, Some(timezone), 2000).unwrap();
        let months = build_graph_buckets(&range, GraphInterval::Month, Some(timezone), 2000).unwrap();
        let local_starts = |buckets: &[DateRange]| {
            buckets
                .iter()
                .map(|bucket| {
                    bucket
                        .start
                        .to_zoned_datetime(TimeZone::get("America/New_York").unwrap())
                        .strftime("%Y-%m-%d %H:%M")
                        .to_string()
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            local_starts(&weeks),
            vec![
                "2024-02-26 00:00",
                "2024-03-04 00:00",
                "2024-03-11 00:00",
                "2024-03-18 00:00",
                "2024-03-25 00:00",
                "2024-04-01 00:00"
            ]
        );
        assert_eq!(local_starts(&months), vec!["2024-02-01 00:00", "2024-03-01 00:00", "2024-04-01 00:00"]);
        assert_eq!(weeks.last().unwrap().end, range.end);
        assert_eq!(months.last().unwrap().end, range.end);
    }

    #[test]
    fn overall_report_includes_start_and_excludes_end() {
        let app = Liwan::new_memory(Config::default()).expect("failed to create app");
        app.seed_database(0).expect("failed to seed app");

        let start = local_datetime("UTC", 2024, 1, 1, 0, 0);
        let middle = local_datetime("UTC", 2024, 1, 2, 12, 0);
        let end = local_datetime("UTC", 2024, 1, 3, 0, 0);

        app.events
            .append(vec![test_event(start), test_event(middle), test_event(end)].into_iter())
            .expect("failed to append events");

        let range = DateRange { start, end };
        let buckets =
            build_graph_buckets(&range, GraphInterval::Day, Some("UTC"), 2000).expect("failed to build buckets");
        let conn = app.events_conn().expect("failed to get events conn");
        let report =
            overall_report(&conn, &["entity-1".to_string()], "pageview", &[range], &[buckets], &[], &Metric::Views)
                .expect("failed to build report")
                .remove(0);

        let values = report.iter().map(|point| point.value).collect::<Vec<_>>();
        assert_eq!(values, vec![1.0, 1.0]);
    }

    #[test]
    fn overall_report_handles_irregular_buckets() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let conn = app.events_conn().unwrap();
        let start = local_datetime("UTC", 2024, 1, 1, 0, 0);
        for (visitor, hours, previous, next, exit) in [
            ("outside", -1, None, None, None),
            ("one", 0, None, Some(60), None),
            ("one", 1, Some(60), None, Some(120)),
            ("gap", 4, None, None, None),
            ("two", 5, None, None, Some(30)),
            ("boundary", 6, None, None, None),
        ] {
            conn.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at, time_from_last_event, time_to_next_event, exited_at)
                 values ('entity-1', ?, 'pageview', ?, ? * interval '1 second', ? * interval '1 second', ?)",
                duckdb::params![visitor, SqlTimestamp(start + SignedDuration::from_hours(hours)), previous, next, exit.map(|seconds| SqlTimestamp(start + SignedDuration::from_hours(hours) + SignedDuration::from_secs(seconds)))],
            ).unwrap();
        }
        let range =
            DateRange { start: start - SignedDuration::from_hours(2), end: start + SignedDuration::from_hours(10) };
        let buckets: Vec<_> = [(0, 2), (5, 6), (7, 8)]
            .into_iter()
            .map(|(from, to)| DateRange {
                start: start + SignedDuration::from_hours(from),
                end: start + SignedDuration::from_hours(to),
            })
            .collect();
        for (metric, expected) in [
            (Metric::Views, vec![2.0, 1.0, 0.0]),
            (Metric::UniqueVisitors, vec![1.0, 1.0, 0.0]),
            (Metric::BounceRate, vec![0.0, 1.0, 0.0]),
            (Metric::AvgTimeOnSite, vec![90.0, 30.0, 0.0]),
        ] {
            let report = overall_report(
                &conn,
                &["entity-1".into()],
                "pageview",
                std::slice::from_ref(&range),
                std::slice::from_ref(&buckets),
                &[],
                &metric,
            )
            .unwrap()
            .remove(0);
            assert_eq!(report.iter().map(|point| point.value).collect::<Vec<_>>(), expected);
        }
    }

    #[test]
    fn build_graph_buckets_rejects_excessive_ranges() {
        let start = local_datetime("UTC", 2024, 1, 1, 0, 0);
        let max_datapoints = 2000;
        let range = DateRange { start, end: start + SignedDuration::from_hours(max_datapoints as i64 + 1) };

        assert!(build_graph_buckets(&range, GraphInterval::Hour, Some("UTC"), max_datapoints).is_err());
    }
}
