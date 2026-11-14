use crate::app::DuckDBConn;
use crate::config::LimitsConfig;
use crate::utils::duckdb::{ParamVec, repeat_vars};
use anyhow::Result;
use chrono::{DateTime, Utc};
use duckdb::params_from_iter;

use super::shared::{build_filter_clause, metric_aggregate_sql, metric_sample_count_sql, range_events_sql};
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

/// Count visitor groups with pageviews in the last five minutes.
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
				e.event = 'pageview' and e.entity_id in ({vars}) and
				e.created_at >= (now()::timestamp - interval '5 minutes');
	"
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let online_users = stmt.query_row(params_from_iter(entities), |row| row.get(0))?;
    Ok(online_users)
}

/// Build all range summaries and the live visitor count in one query.
pub fn overall_stats(
    conn: &DuckDBConn,
    entities: &[String],
    event: &str,
    ranges: &[DateRange],
    filters: &[DimensionFilter],
    limits: &LimitsConfig,
) -> Result<(Vec<ReportStats>, u64)> {
    if entities.is_empty() || ranges.is_empty() {
        return Ok((vec![ReportStats::default(); ranges.len()], 0));
    }

    let entity_vars = repeat_vars(entities.len());
    let (filters_sql, filters_params) = build_filter_clause(filters)?;

    let metric_total = metric_aggregate_sql(Metric::Views, "sd");
    let metric_unique_visitors = metric_aggregate_sql(Metric::UniqueVisitors, "sd");
    let (metric_bounce_rate, metric_avg_time_on_site) = if event == "pageview" {
        (metric_aggregate_sql(Metric::BounceRate, "sd"), metric_aggregate_sql(Metric::AvgTimeOnSite, "sd"))
    } else {
        ("null::double".into(), "null::double".into())
    };
    let (bounce_samples, time_samples) = if event == "pageview" {
        (metric_sample_count_sql(Metric::BounceRate, "sd"), metric_sample_count_sql(Metric::AvgTimeOnSite, "sd"))
    } else {
        ("0::ubigint".into(), "0::ubigint".into())
    };

    let mut params = ParamVec::new();
    let range_events = range_events_sql(ranges, &mut params);
    let range_count = ranges.len();
    params.push(event);
    params.extend(entities);
    params.extend_from_params(filters_params);
    params.extend(entities);

    let query = format!(
        "--sql
		with
			range_events as ({range_events}),
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
			range_stats as (
				select range_idx,
					{metric_total} as total_views,
					{metric_unique_visitors} as unique_visitors,
					{metric_bounce_rate} as bounce_rate,
					{metric_avg_time_on_site} as avg_time_on_site,
					{bounce_samples} as bounce_samples,
					{time_samples} as time_samples
				from session_data sd
				group by range_idx
			),
			online as (
				select count(distinct visitor_group_id) as current_visitors
				from events
				where event = 'pageview' and entity_id in ({entity_vars})
					and created_at >= (now()::timestamp - interval '5 minutes')
			)
		select
			coalesce(s.total_views, 0), coalesce(s.unique_visitors, 0),
			s.bounce_rate, s.avg_time_on_site,
			coalesce(s.bounce_samples, 0), coalesce(s.time_samples, 0),
			online.current_visitors
		from range({range_count}) r(range_idx)
		left join range_stats s using (range_idx)
		cross join online
		order by r.range_idx;
	"
    );

    let mut stmt = conn.prepare_cached(&query)?;
    let mut rows = stmt.query(duckdb::params_from_iter(params))?;
    let mut reports = Vec::with_capacity(ranges.len());
    let mut current_visitors = 0;
    while let Some(row) = rows.next()? {
        let mut stats = ReportStats {
            total_views: row.get(0)?,
            unique_visitors: row.get(1)?,
            bounce_rate: row.get(2)?,
            avg_time_on_site: row.get(3)?,
        };
        let bounce_samples: u64 = row.get(4)?;
        let time_samples: u64 = row.get(5)?;
        // Empty ranges retain the single-range aggregate defaults when thresholds are disabled.
        if event == "pageview" {
            stats.bounce_rate = stats.bounce_rate.or(Some(1.0));
            stats.avg_time_on_site = stats.avg_time_on_site.or(Some(0.0));
        }
        if bounce_samples < limits.report_min_bounce_rate_sessions {
            stats.bounce_rate = None;
        }
        if time_samples < limits.report_min_avg_time_on_site_samples {
            stats.avg_time_on_site = None;
        }
        reports.push(stats);
        current_visitors = row.get(6)?;
    }
    Ok((reports, current_visitors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Liwan;
    use crate::config::Config;
    use chrono::Duration;

    #[test]
    fn online_users_counts_only_pageviews() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let conn = app.events_conn().unwrap();
        for (visitor, event) in [("visitor-1", "pageview"), ("visitor-2", "signup")] {
            conn.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at) values (?, ?, ?, ?)",
                duckdb::params!["entity-1", visitor, event, Utc::now()],
            )
            .unwrap();
        }

        assert_eq!(online_users(&conn, &["entity-1".into()]).unwrap(), 1);
        let now = Utc::now();
        let ranges = [DateRange { start: now - Duration::days(2), end: now - Duration::days(1) }];
        let (stats, current_visitors) =
            overall_stats(&conn, &["entity-1".into()], "signup", &ranges, &[], &LimitsConfig::default()).unwrap();
        assert_eq!(stats[0].total_views, 0);
        assert_eq!(current_visitors, 1);
    }

    #[test]
    fn overall_stats_uses_terminal_event_exit_duration() {
        let app = Liwan::new_memory(Config::default()).expect("failed to create app");
        let created_at = Utc::now() - Duration::minutes(5);
        let exited_at = created_at + Duration::seconds(90);
        let conn = app.events_conn().expect("failed to get event connection");
        conn.execute(
            "insert into events (entity_id, visitor_group_id, event, created_at, fqdn, path, exited_at) values (?, ?, ?, ?, ?, ?, ?)",
            duckdb::params!["entity-1", "visitor-1", "pageview", created_at, "example.com", "/", exited_at],
        )
        .expect("failed to insert event");

        let range = DateRange { start: created_at - Duration::minutes(1), end: exited_at + Duration::minutes(1) };
        let ranges = [range];
        let limits = LimitsConfig {
            report_min_bounce_rate_sessions: 0,
            report_min_avg_time_on_site_samples: 0,
            ..Default::default()
        };
        let stats = overall_stats(&conn, &["entity-1".to_string()], "pageview", &ranges, &[], &limits)
            .expect("failed to build stats")
            .0
            .remove(0);

        assert_eq!(stats.avg_time_on_site, Some(90.0));

        let limits = LimitsConfig {
            report_min_bounce_rate_sessions: 1,
            report_min_avg_time_on_site_samples: 2,
            ..Default::default()
        };
        let stats = overall_stats(&conn, &["entity-1".to_string()], "pageview", &ranges, &[], &limits)
            .expect("failed to build stats with separate limits")
            .0
            .remove(0);
        assert_eq!(stats.bounce_rate, Some(1.0));
        assert_eq!(stats.avg_time_on_site, None);

        let stats = overall_stats(&conn, &["entity-1".to_string()], "pageview", &ranges, &[], &LimitsConfig::default())
            .expect("failed to build stats with default limits")
            .0
            .remove(0);
        assert_eq!(stats.bounce_rate, None);
        assert_eq!(stats.avg_time_on_site, None);
    }
}
