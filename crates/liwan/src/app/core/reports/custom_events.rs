use anyhow::Result;
use schemars::JsonSchema;
use serde::Serialize;

use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};

use super::shared::{build_filter_clause, range_events_sql};
use super::{DateRange, DimensionFilter};

/// Counts for one custom event in the selected range.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventRow {
    pub name: String,
    /// Event completions, or null when the event has no matches in this range.
    pub completions: Option<u64>,
    /// Distinct visitor groups, or null when the event has no matches in this range.
    pub uniques: Option<u64>,
}

/// Custom events visible in a project report.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventsReport {
    pub rows: Vec<CustomEventRow>,
    pub truncated: bool,
}

/// Select top events per range and return their counts across every requested range.
pub fn custom_events_report(
    conn: &DuckDBConn,
    entities: &[String],
    ranges: &[DateRange],
    filters: &[DimensionFilter],
    limit: usize,
) -> Result<Vec<CustomEventsReport>> {
    let mut reports: Vec<_> =
        ranges.iter().map(|_| CustomEventsReport { rows: Vec::new(), truncated: false }).collect();
    if entities.is_empty() || ranges.is_empty() {
        return Ok(reports);
    }

    let entity_vars = repeat_vars(entities.len());
    let (filters_sql, filters_params) = build_filter_clause(filters)?;
    let mut params = ParamVec::new();
    let range_events = range_events_sql(ranges, &mut params);
    let range_count = ranges.len();
    params.extend(entities);
    params.extend_from_params(filters_params);

    let sql = format!(
        "with range_events as ({range_events}),
         aggregates as materialized (
             select range_idx, event, count(*) as completions, count(distinct visitor_group_id) as uniques
             from range_events
             where event <> 'pageview' and entity_id in ({entity_vars}) {filters_sql}
             group by range_idx, event
         ),
         selected_events as (
             select distinct event from aggregates
             qualify row_number() over (partition by range_idx order by completions desc, event) <= {limit}
         ),
         totals as (
             select range_idx, count(*) as event_count from aggregates group by range_idx
         )
         select r.range_idx, k.event, a.completions, a.uniques,
             coalesce(t.event_count, 0) > count(a.event) over (partition by r.range_idx) as truncated
         from range({range_count}) r(range_idx)
         left join selected_events k on true
         left join aggregates a on a.range_idx = r.range_idx and a.event = k.event
         left join totals t on t.range_idx = r.range_idx
         order by r.range_idx, a.completions desc nulls last, k.event"
    );
    let mut stmt = conn.prepare_cached(&sql)?;
    let mut rows = stmt.query(duckdb::params_from_iter(params))?;
    while let Some(row) = rows.next()? {
        let report = &mut reports[row.get::<_, usize>(0)?];
        report.truncated = row.get(4)?;
        if let Some(name) = row.get(1)? {
            report.rows.push(CustomEventRow { name, completions: row.get(2)?, uniques: row.get(3)? });
        }
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Liwan;
    use crate::app::models::FilterType;
    use crate::app::reports::Dimension;
    use crate::config::Config;
    use chrono::{Duration, Utc};

    #[test]
    fn counts_only_matching_custom_events() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let conn = app.events_conn().unwrap();
        let now = Utc::now();
        for (entity, visitor, event, time) in [
            ("a", "one", "pageview", now),
            ("a", "one", "signup", now),
            ("a", "one", "signup", now),
            ("a", "two", "signup", now),
            ("a", "two", "download", now),
            ("b", "three", "signup", now),
            ("a", "old", "outside", now - Duration::days(2)),
        ] {
            conn.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at, path) values (?, ?, ?, ?, ?)",
                duckdb::params![entity, visitor, event, time, "/pricing"],
            )
            .unwrap();
        }
        let range = DateRange { start: now - Duration::hours(1), end: now + Duration::hours(1) };
        let ranges = [range];
        let report = custom_events_report(&conn, &["a".into()], &ranges, &[], 1).unwrap().remove(0);
        assert!(report.truncated);
        assert_eq!(report.rows[0].name, "signup");
        assert_eq!(report.rows[0].completions, Some(3));
        assert_eq!(report.rows[0].uniques, Some(2));

        let report = custom_events_report(&conn, &["a".into(), "b".into()], &ranges, &[], 10).unwrap().remove(0);
        assert!(!report.truncated);
        assert_eq!(report.rows.len(), 2);
        assert_eq!(report.rows[0].completions, Some(4));
        assert_eq!(report.rows[0].uniques, Some(3));

        let report = custom_events_report(&conn, &["a".into()], &ranges, &[], 0).unwrap().remove(0);
        assert!(report.truncated);
        assert!(report.rows.is_empty());

        let empty = DateRange { start: now - Duration::hours(3), end: now - Duration::hours(2) };
        assert!(custom_events_report(&conn, &["a".into()], &[empty], &[], 10).unwrap()[0].rows.is_empty());

        let filter = DimensionFilter {
            dimension: Dimension::Path,
            key: None,
            filter_type: FilterType::Equal,
            inversed: None,
            strict: None,
            value: Some("/missing".into()),
        };
        assert!(custom_events_report(&conn, &["a".into()], &ranges, &[filter], 10).unwrap()[0].rows.is_empty());
    }
}
