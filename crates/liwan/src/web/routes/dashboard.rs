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
    data: reports::ReportGraph,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct StatsRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
    event: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct CustomEventsRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
}

async fn project_custom_events_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeAuth(user): MaybeAuth,
    Json(req): Json<CustomEventsRequest>,
) -> ApiResult<Json<reports::CustomEventsReport>> {
    let (entities, display) = report_project(&app, project_id, user).await?;
    if display.custom_events_display() == DisplayOverride::Hide {
        http_bail!(StatusCode::BAD_REQUEST, "Custom events are hidden for this project")
    }
    reports::validate_request(&req.range, &req.filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;
    if req.filters.iter().any(DimensionFilter::is_session_page_filter) {
        http_bail!(StatusCode::BAD_REQUEST, "Entry and exit page filters are not supported for custom events")
    }
    let limit = app.config.limits.report_max_dimension_results;
    let report =
        run_report(&app, move |conn| reports::custom_events_report(conn, &entities, &req.range, &req.filters, limit))
            .await?;
    Ok(Json(report))
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct GraphRequest {
    range: DateRange,
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
    stats: ReportStats,
    stats_prev: ReportStats,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct DimensionRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
    metric: Metric,
    dimension: Dimension,
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
    data: Vec<DimensionTableRow>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct DimensionTableRow {
    dimension_value: String,
    value: f64,
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

    reports::validate_request(&req.range, &req.filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;
    let collection = app.settings.resolved_for_entities(&entities);
    let event = report_event(req.event.as_deref(), &req.filters, Some(req.metric), display.custom_events_display())?;

    if display.is_metric_hidden(&collection, req.metric) {
        http_bail!(StatusCode::BAD_REQUEST, "Metric is hidden for this project")
    }

    let buckets = reports::build_graph_buckets(
        &req.range,
        req.interval,
        req.timezone.as_deref(),
        app.config.limits.report_max_datapoints,
    )
    .http_status(StatusCode::BAD_REQUEST)?;

    let report = run_report(&app, move |conn| {
        reports::overall_report(conn, &entities, &event, &req.range, &buckets, &req.filters, &req.metric)
    })
    .await?;

    Ok(Json(GraphResponse { data: report }))
}

async fn project_stats_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeAuth(user): MaybeAuth,
    Json(req): Json<StatsRequest>,
) -> ApiResult<Json<StatsResponse>> {
    let (entities, display) = report_project(&app, project_id, user).await?;
    reports::validate_request(&req.range, &req.filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;
    let event = report_event(req.event.as_deref(), &req.filters, None, display.custom_events_display())?;

    let (entities2, entities3) = (entities.clone(), entities.clone());
    let collection = app.settings.resolved_for_entities(&entities);

    let req2 = req.clone();
    let event_prev = event.clone();
    let previous_range = req.range.prev().http_status(StatusCode::BAD_REQUEST)?;

    let (mut stats, mut stats_prev) = tokio::try_join!(
        run_report(&app, move |conn| reports::overall_stats(conn, &entities, &event, &req.range, &req.filters)),
        run_report(&app, move |conn| {
            reports::overall_stats(conn, &entities2, &event_prev, &previous_range, &req2.filters)
        }),
    )?;

    if display.is_metric_hidden(&collection, Metric::BounceRate) {
        stats.bounce_rate = None;
        stats_prev.bounce_rate = None;
    }
    if display.is_metric_hidden(&collection, Metric::AvgTimeOnSite) {
        stats.avg_time_on_site = None;
        stats_prev.avg_time_on_site = None;
    }

    let online = run_report(&app, move |conn| reports::online_users(conn, &entities3)).await?;

    Ok(Json(StatsResponse { current_visitors: online, stats, stats_prev }))
}

async fn project_detailed_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
    Path(project_id): Path<String>,
    Json(req): Json<DimensionRequest>,
) -> ApiResult<Json<DimensionResponse>> {
    let (entities, display) = report_project(&app, project_id, user).await?;
    reports::validate_request(&req.range, &req.filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;
    let collection = app.settings.resolved_for_entities(&entities);
    let event = report_event(req.event.as_deref(), &req.filters, Some(req.metric), display.custom_events_display())?;
    if event != "pageview" && matches!(req.dimension, Dimension::UrlEntry | Dimension::UrlExit) {
        http_bail!(StatusCode::BAD_REQUEST, "Entry and exit pages are not supported for custom events")
    }

    if display.is_metric_hidden(&collection, req.metric) {
        http_bail!(StatusCode::BAD_REQUEST, "Metric is hidden for this project")
    }
    if display.is_dimension_hidden(&collection, req.dimension) {
        http_bail!(StatusCode::BAD_REQUEST, "Dimension is hidden for this project")
    }

    let max_results = app.config.limits.report_max_dimension_results;
    let stats = run_report(&app, move |conn| {
        reports::dimension_report(
            conn,
            &entities,
            &event,
            &req.range,
            &req.dimension,
            &req.filters,
            &req.metric,
            max_results,
        )
    })
    .await?;

    let mut data = Vec::new();
    for (key, value) in stats {
        match req.dimension {
            Dimension::Referrer => {
                let display_name = crate::utils::referrer::get_referer_name(&key);
                let icon = if let Some(referrer) = &display_name {
                    crate::utils::referrer::get_referer_icon(referrer)
                } else {
                    None
                };
                data.push(DimensionTableRow { dimension_value: key, value, display_name, icon });
            }
            Dimension::Browser => {
                let display_name = match key.as_str() {
                    "Edge" => Some("Microsoft Edge".to_string()),
                    _ => None,
                };
                data.push(DimensionTableRow { dimension_value: key, value, display_name, icon: None });
            }
            Dimension::Country => {
                let display_name = crate::utils::geo::get_country_name(&key);
                data.push(DimensionTableRow { dimension_value: key, value, display_name, icon: None });
            }
            Dimension::City => {
                let (country, city) = key
                    .clone()
                    .split_at_checked(2)
                    .map_or((None, None), |(a, b)| (Some(a.to_string()), Some(b.to_string())));
                let city = city.filter(|city| !city.is_empty());
                data.push(DimensionTableRow { dimension_value: key, value, display_name: city, icon: country });
            }
            Dimension::ScreenWidth | Dimension::Orientation => {
                let display_name =
                    key.chars().next().map(|c| c.to_uppercase().collect::<String>() + &key[c.len_utf8()..]);
                data.push(DimensionTableRow { dimension_value: key, value, display_name, icon: None });
            }
            _ => {
                data.push(DimensionTableRow { dimension_value: key, value, display_name: None, icon: None });
            }
        }
    }

    Ok(Json(DimensionResponse { data }))
}
