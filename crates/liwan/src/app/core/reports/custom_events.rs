use anyhow::Result;
use schemars::JsonSchema;
use serde::Serialize;

use crate::app::DuckDBConn;
use crate::utils::duckdb::{ParamVec, repeat_vars};

use super::shared::build_filter_clause;
use super::{DateRange, DimensionFilter};

/// Counts for one custom event in the selected range.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventRow {
    pub name: String,
    pub completions: u64,
    pub uniques: u64,
}

/// Custom events visible in a project report.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomEventsReport {
    pub rows: Vec<CustomEventRow>,
    pub truncated: bool,
}

/// Summarize custom events for a project's entities.
pub fn custom_events_report(
    conn: &DuckDBConn,
    entities: &[String],
    range: &DateRange,
    filters: &[DimensionFilter],
    limit: usize,
) -> Result<CustomEventsReport> {
    if entities.is_empty() {
        return Ok(CustomEventsReport { rows: Vec::new(), truncated: false });
    }

    let entity_vars = repeat_vars(entities.len());
    let (filters_sql, filters_params) = build_filter_clause(filters)?;
    let mut params = ParamVec::new();
    params.push(range.start);
    params.push(range.end);
    params.extend(entities);
    params.extend_from_params(filters_params);

    let sql = format!(
        "select event, count(*) as completions, count(distinct visitor_group_id) as uniques
         from events
         where event <> 'pageview' and created_at >= ?::timestamp and created_at < ?::timestamp
             and entity_id in ({entity_vars}) {filters_sql}
         group by event
         order by completions desc, event
         limit {}",
        limit.saturating_add(1)
    );
    let rows = conn
        .prepare_cached(&sql)?
        .query_map(duckdb::params_from_iter(params), |row| {
            Ok(CustomEventRow { name: row.get(0)?, completions: row.get(1)?, uniques: row.get(2)? })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let truncated = rows.len() > limit;
    Ok(CustomEventsReport { rows: rows.into_iter().take(limit).collect(), truncated })
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
        let report = custom_events_report(&conn, &["a".into()], &range, &[], 1).unwrap();
        assert!(report.truncated);
        assert_eq!(report.rows[0].name, "signup");
        assert_eq!(report.rows[0].completions, 3);
        assert_eq!(report.rows[0].uniques, 2);

        let empty = DateRange { start: now - Duration::hours(3), end: now - Duration::hours(2) };
        assert!(custom_events_report(&conn, &["a".into()], &empty, &[], 10).unwrap().rows.is_empty());

        let filter = DimensionFilter {
            dimension: Dimension::Path,
            key: None,
            filter_type: FilterType::Equal,
            inversed: None,
            strict: None,
            value: Some("/missing".into()),
        };
        assert!(custom_events_report(&conn, &["a".into()], &range, &[filter], 10).unwrap().rows.is_empty());
    }
}
