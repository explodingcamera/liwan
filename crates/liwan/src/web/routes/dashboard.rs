use crate::app::DuckDBConn;
use crate::app::models::{DisplayOverride, ProjectDisplaySettings, User};
use crate::app::reports::{self, DateRange, Dimension, DimensionFilter, GraphInterval, Metric, ReportStats};
use crate::utils::validate::can_view_project;
use crate::web::RouterState;
use crate::web::session::MaybeAuth;
use crate::web::webext::{ApiResult, AxumErrExt, http_bail};

use aide::axum::{ApiRouter, routing::*};
use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::task::spawn_blocking;

async fn report_project(
    app: &RouterState,
    project_id: String,
    user: Option<User>,
) -> ApiResult<(Vec<String>, ProjectDisplaySettings)> {
    let app = app.app.clone();
    spawn_blocking(move || {
        let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if !can_view_project(&project, user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }
        let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        let display = app.project_settings.get(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok((entities, display))
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
}

async fn run_report<T, F>(app: &RouterState, report: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce(&DuckDBConn) -> anyhow::Result<T> + Send + 'static,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(app.config.limits.report_timeout_seconds);
    let permit = tokio::time::timeout_at(deadline, app.report_permits.clone().acquire_owned())
        .await
        .http_err("Timed out waiting to run report", StatusCode::SERVICE_UNAVAILABLE)?
        .http_status(StatusCode::SERVICE_UNAVAILABLE)?;
    let app = app.app.clone();
    // Keep the deadline active even if the request or a sibling report is cancelled.
    tokio::spawn(async move {
        let conn = spawn_blocking(move || {
            app.events_pool.get_timeout(deadline.saturating_duration_since(tokio::time::Instant::now()))
        })
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Timed out waiting for a report connection", StatusCode::SERVICE_UNAVAILABLE)?;
        if tokio::time::Instant::now() >= deadline {
            http_bail!(StatusCode::SERVICE_UNAVAILABLE, "Timed out waiting to run report")
        }
        let interrupt = conn.interrupt_handle();
        let mut task = spawn_blocking(move || {
            let _permit = permit;
            let result = report(&conn);
            (conn, result)
        });

        match tokio::time::timeout_at(deadline, &mut task).await {
            Ok(result) => {
                result.http_status(StatusCode::INTERNAL_SERVER_ERROR)?.1.http_status(StatusCode::INTERNAL_SERVER_ERROR)
            }
            Err(_) => {
                interrupt.interrupt();
                task.abort();
                http_bail!(StatusCode::GATEWAY_TIMEOUT, "Report query timed out")
            }
        }
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
}

pub fn router() -> ApiRouter<RouterState> {
    ApiRouter::new()
        .api_route("/project/{project_id}/earliest", get(project_earliest_handler))
        .api_route("/project/{project_id}/graph", post(project_graph_handler))
        .api_route("/project/{project_id}/stats", post(project_stats_handler))
        .api_route("/project/{project_id}/dimension", post(project_detailed_handler))
        .api_route("/project/{project_id}/custom-events", post(project_custom_events_handler))
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct GraphResponse {
    /// Reports in the same order as the requested ranges.
    reports: Vec<reports::ReportGraph>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct StatsRequest {
    /// Date ranges to report, in response order. Limits apply to each range.
    #[schemars(length(min = 1, max = 4))]
    ranges: Vec<DateRange>,
    filters: Vec<DimensionFilter>,
    event: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct CustomEventsRequest {
    /// Date ranges to report, in response order. Top events are selected per range.
    #[schemars(length(min = 1, max = 4))]
    ranges: Vec<DateRange>,
    filters: Vec<DimensionFilter>,
}

#[derive(Serialize, JsonSchema)]
struct CustomEventsResponse {
    /// Reports in range order, each containing the events selected by any range.
    reports: Vec<reports::CustomEventsReport>,
}

fn validate_ranges(app: &RouterState, ranges: &[DateRange], filters: &[DimensionFilter]) -> ApiResult<()> {
    if ranges.is_empty() || ranges.len() > 4 {
        http_bail!(StatusCode::BAD_REQUEST, "Reports require between 1 and 4 ranges")
    }
    for range in ranges {
        reports::validate_request(range, filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;
    }
    Ok(())
}

async fn project_custom_events_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeAuth(user): MaybeAuth,
    Json(req): Json<CustomEventsRequest>,
) -> ApiResult<Json<CustomEventsResponse>> {
    let (entities, display) = report_project(&app, project_id, user).await?;
    if display.custom_events_display() == DisplayOverride::Hide {
        http_bail!(StatusCode::BAD_REQUEST, "Custom events are hidden for this project")
    }
    validate_ranges(&app, &req.ranges, &req.filters)?;
    if req.filters.iter().any(DimensionFilter::is_session_page_filter) {
        http_bail!(StatusCode::BAD_REQUEST, "Entry and exit page filters are not supported for custom events")
    }
    let limit = app.config.limits.report_max_dimension_results;
    let reports =
        run_report(&app, move |conn| reports::custom_events_report(conn, &entities, &req.ranges, &req.filters, limit))
            .await?;
    Ok(Json(CustomEventsResponse { reports }))
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct GraphRequest {
    /// Date ranges to report, in response order. Limits apply to each range.
    #[schemars(length(min = 1, max = 4))]
    ranges: Vec<DateRange>,
    filters: Vec<DimensionFilter>,
    interval: GraphInterval,
    timezone: Option<String>,
    metric: Metric,
    event: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct StatsResponse {
    current_visitors: u64,
    /// Reports in the same order as the requested ranges.
    reports: Vec<ReportStats>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct DimensionRequest {
    /// Date ranges to report, in response order. Top dimensions are selected per range.
    #[schemars(length(min = 1, max = 4))]
    ranges: Vec<DateRange>,
    filters: Vec<DimensionFilter>,
    metric: Metric,
    dimension: Dimension,
    /// Property key whose values are listed; without it, property keys are listed
    property_key: Option<String>,
    event: Option<String>,
}

fn report_event(
    event: Option<&str>,
    filters: &[DimensionFilter],
    metric: Option<Metric>,
    display: DisplayOverride,
) -> ApiResult<String> {
    let event = event.unwrap_or("pageview");
    if event.trim().is_empty() || event.len() > 255 {
        http_bail!(StatusCode::BAD_REQUEST, "Invalid event name")
    }
    if event != "pageview" && display == DisplayOverride::Hide {
        http_bail!(StatusCode::BAD_REQUEST, "Custom events are hidden for this project")
    }
    if event != "pageview"
        && (filters.iter().any(DimensionFilter::is_session_page_filter)
            || matches!(metric, Some(Metric::BounceRate | Metric::AvgTimeOnSite)))
    {
        http_bail!(StatusCode::BAD_REQUEST, "Pageview session metrics and filters are not supported for custom events")
    }
    Ok(event.to_owned())
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct DimensionResponse {
    /// Reports in range order, each containing the dimensions selected by any range.
    reports: Vec<Vec<DimensionTableRow>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct DimensionTableRow {
    dimension_value: String,
    /// Metric value, or null when there is no data or not enough samples for a session metric.
    value: Option<f64>,
    display_name: Option<String>,
    icon: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct EarliestResponse {
    earliest: Option<DateTime<Utc>>,
}

async fn project_earliest_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
    Path(project_id): Path<String>,
) -> ApiResult<Json<EarliestResponse>> {
    let (entities, _) = report_project(&app, project_id, user).await?;
    let earliest = run_report(&app, move |conn| reports::earliest_timestamp(conn, &entities)).await?;

    Ok(Json(EarliestResponse { earliest }))
}

async fn project_graph_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeAuth(user): MaybeAuth,
    Json(req): Json<GraphRequest>,
) -> ApiResult<Json<GraphResponse>> {
    let (entities, display) = report_project(&app, project_id, user).await?;

    validate_ranges(&app, &req.ranges, &req.filters)?;
    let collection = app.settings.resolved_for_entities(&entities);
    let event = report_event(req.event.as_deref(), &req.filters, Some(req.metric), display.custom_events_display())?;

    if display.is_metric_hidden(&collection, req.metric) {
        http_bail!(StatusCode::BAD_REQUEST, "Metric is hidden for this project")
    }

    let buckets = req
        .ranges
        .iter()
        .map(|range| {
            reports::build_graph_buckets(
                range,
                req.interval,
                req.timezone.as_deref(),
                app.config.limits.report_max_datapoints,
            )
        })
        .collect::<anyhow::Result<Vec<_>>>()
        .http_status(StatusCode::BAD_REQUEST)?;

    let reports = run_report(&app, move |conn| {
        reports::overall_report(conn, &entities, &event, &req.ranges, &buckets, &req.filters, &req.metric)
    })
    .await?;

    Ok(Json(GraphResponse { reports }))
}

async fn project_stats_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeAuth(user): MaybeAuth,
    Json(req): Json<StatsRequest>,
) -> ApiResult<Json<StatsResponse>> {
    let (entities, display) = report_project(&app, project_id, user).await?;
    validate_ranges(&app, &req.ranges, &req.filters)?;
    let event = report_event(req.event.as_deref(), &req.filters, None, display.custom_events_display())?;

    let collection = app.settings.resolved_for_entities(&entities);
    let limits = app.config.limits.clone();
    let response = run_report(&app, move |conn| {
        let (mut reports, current_visitors) =
            reports::overall_stats(conn, &entities, &event, &req.ranges, &req.filters, &limits)?;
        for stats in &mut reports {
            if display.is_metric_hidden(&collection, Metric::BounceRate) {
                stats.bounce_rate = None;
            }
            if display.is_metric_hidden(&collection, Metric::AvgTimeOnSite) {
                stats.avg_time_on_site = None;
            }
        }
        Ok(StatsResponse { current_visitors, reports })
    })
    .await?;
    Ok(Json(response))
}

async fn project_detailed_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
    Path(project_id): Path<String>,
    Json(req): Json<DimensionRequest>,
) -> ApiResult<Json<DimensionResponse>> {
    let (entities, display) = report_project(&app, project_id, user).await?;
    validate_ranges(&app, &req.ranges, &req.filters)?;
    let collection = app.settings.resolved_for_entities(&entities);
    let event = report_event(req.event.as_deref(), &req.filters, Some(req.metric), display.custom_events_display())?;
    match (req.dimension, req.property_key.as_deref()) {
        (Dimension::Property, Some(key)) => {
            reports::validate_property_key(key).http_status(StatusCode::BAD_REQUEST)?;
        }
        (_, Some(_)) => http_bail!(StatusCode::BAD_REQUEST, "Only property reports can have a property key"),
        _ => {}
    }
    if event != "pageview" && matches!(req.dimension, Dimension::UrlEntry | Dimension::UrlExit) {
        http_bail!(StatusCode::BAD_REQUEST, "Entry and exit pages are not supported for custom events")
    }

    if display.is_metric_hidden(&collection, req.metric) {
        http_bail!(StatusCode::BAD_REQUEST, "Metric is hidden for this project")
    }
    let (events, entity_ids) = (app.app.events.clone(), entities.clone());
    let data = spawn_blocking(move || events.custom_data(&entity_ids))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    if display.is_dimension_hidden(&collection, data, req.dimension) {
        http_bail!(StatusCode::BAD_REQUEST, "Dimension is hidden for this project")
    }

    let max_results = app.config.limits.report_max_dimension_results;
    let min_samples = match req.metric {
        Metric::BounceRate => app.config.limits.report_min_bounce_rate_sessions,
        Metric::AvgTimeOnSite => app.config.limits.report_min_avg_time_on_site_samples,
        _ => 0,
    };
    let reports = run_report(&app, move |conn| {
        let tables = reports::dimension_report(
            conn,
            &entities,
            &event,
            &req.ranges,
            &req.dimension,
            req.property_key.as_deref(),
            &req.filters,
            &req.metric,
            max_results,
            min_samples,
        )?;
        let mut reports = Vec::with_capacity(tables.len());
        for stats in tables {
            let mut data = Vec::with_capacity(stats.len());
            for (key, value) in stats {
                let (display_name, icon) = match req.dimension {
                    Dimension::Referrer => {
                        let display_name = crate::utils::referrer::get_referer_name(&key);
                        let icon = display_name.as_deref().and_then(crate::utils::referrer::get_referer_icon);
                        (display_name, icon)
                    }
                    Dimension::Browser => {
                        let display_name = match key.as_str() {
                            "Edge" => Some("Microsoft Edge".to_string()),
                            _ => None,
                        };
                        (display_name, None)
                    }
                    Dimension::Country => (crate::utils::geo::get_country_name(&key), None),
                    Dimension::City => {
                        let (country, city) = key
                            .split_at_checked(2)
                            .map_or((None, None), |(a, b)| (Some(a.to_string()), Some(b.to_string())));
                        (city.filter(|city| !city.is_empty()), country)
                    }
                    Dimension::ScreenWidth | Dimension::Orientation => {
                        let display_name =
                            key.chars().next().map(|c| c.to_uppercase().collect::<String>() + &key[c.len_utf8()..]);
                        (display_name, None)
                    }
                    _ => (None, None),
                };
                data.push(DimensionTableRow { dimension_value: key, value, display_name, icon });
            }

            reports.push(data);
        }
        Ok(reports)
    })
    .await?;
    Ok(Json(DimensionResponse { reports }))
}
