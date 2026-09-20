use crate::app::DuckDBConn;
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

async fn run_report<T, F>(app: &RouterState, report: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce(&DuckDBConn) -> anyhow::Result<T> + Send + 'static,
{
    let deadline = Duration::from_secs(app.config.limits.report_timeout_seconds);
    let permit = tokio::time::timeout(deadline, app.report_permits.clone().acquire_owned())
        .await
        .http_err("Timed out waiting to run report", StatusCode::SERVICE_UNAVAILABLE)?
        .http_status(StatusCode::SERVICE_UNAVAILABLE)?;
    let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let interrupt = conn.interrupt_handle();
    let mut task = spawn_blocking(move || {
        let _permit = permit;
        let result = report(&conn);
        (conn, result)
    });

    match tokio::time::timeout(deadline, &mut task).await {
        Ok(result) => {
            result.http_status(StatusCode::INTERNAL_SERVER_ERROR)?.1.http_status(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(_) => {
            interrupt.interrupt();
            http_bail!(StatusCode::GATEWAY_TIMEOUT, "Report query timed out")
        }
    }
}

pub fn router() -> ApiRouter<RouterState> {
    ApiRouter::new()
        .api_route("/project/{project_id}/earliest", get(project_earliest_handler))
        .api_route("/project/{project_id}/graph", post(project_graph_handler))
        .api_route("/project/{project_id}/stats", post(project_stats_handler))
        .api_route("/project/{project_id}/dimension", post(project_detailed_handler))
        .api_route("/project/{project_id}/sessions", post(project_sessions_handler))
        .api_route(
            "/project/{project_id}/sessions/{visitor_group_id}/timeline",
            post(project_session_timeline_handler),
        )
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct GraphResponse {
    data: reports::ReportGraph,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct StatsRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct GraphRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
    interval: GraphInterval,
    timezone: Option<String>,
    metric: Metric,
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
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct DimensionResponse {
    data: Vec<DimensionTableRow>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct SessionsRequest {
    range: DateRange,
    #[serde(default = "default_sessions_limit")]
    limit: usize,
}

fn default_sessions_limit() -> usize {
    50
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct SessionsResponse {
    data: Vec<reports::SessionRow>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct SessionTimelineRequest {
    range: DateRange,
    #[serde(default = "default_timeline_limit")]
    limit: usize,
}

fn default_timeline_limit() -> usize {
    200
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct SessionTimelineResponse {
    data: Vec<reports::SessionEvent>,
}

async fn project_sessions_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
    Path(project_id): Path<String>,
    Json(req): Json<SessionsRequest>,
) -> ApiResult<Json<SessionsResponse>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    if !can_view_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    reports::validate_request(&req.range, &[], &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;
    let limit = req.limit.clamp(1, 200);

    let data = run_report(&app, move |conn| {
        reports::session_list_report(conn, &entities, &req.range, limit)
    })
    .await?;

    Ok(Json(SessionsResponse { data }))
}

async fn project_session_timeline_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
    Path((project_id, visitor_group_id)): Path<(String, String)>,
    Json(req): Json<SessionTimelineRequest>,
) -> ApiResult<Json<SessionTimelineResponse>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    if !can_view_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    reports::validate_request(&req.range, &[], &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;
    let limit = req.limit.clamp(1, 1000);

    let data = run_report(&app, move |conn| {
        reports::session_timeline_report(conn, &entities, &visitor_group_id, &req.range, limit)
    })
    .await?;

    Ok(Json(SessionTimelineResponse { data }))
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
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;

    if !can_view_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let earliest = run_report(&app, move |conn| reports::earliest_timestamp(conn, &entities)).await?;

    Ok(Json(EarliestResponse { earliest }))
}

async fn project_graph_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeAuth(user): MaybeAuth,
    Json(req): Json<GraphRequest>,
) -> ApiResult<Json<GraphResponse>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::IM_A_TEAPOT)?;
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    if !can_view_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    reports::validate_request(&req.range, &req.filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;

    if app.is_metric_hidden(&project.id, &entities, req.metric) {
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
        reports::overall_report(conn, &entities, "pageview", &req.range, &buckets, &req.filters, &req.metric)
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
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    if !can_view_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }
    reports::validate_request(&req.range, &req.filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;

    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let (entities2, entities3) = (entities.clone(), entities.clone());

    let req2 = req.clone();

    let (mut stats, mut stats_prev) = tokio::try_join!(
        run_report(&app, move |conn| reports::overall_stats(conn, &entities, "pageview", &req.range, &req.filters)),
        run_report(&app, move |conn| {
            reports::overall_stats(conn, &entities2, "pageview", &req2.range.prev(), &req2.filters)
        }),
    )?;

    if app.is_metric_hidden(&project.id, &entities3, Metric::BounceRate) {
        stats.bounce_rate = None;
        stats_prev.bounce_rate = None;
    }
    if app.is_metric_hidden(&project.id, &entities3, Metric::AvgTimeOnSite) {
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
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    if !can_view_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }
    reports::validate_request(&req.range, &req.filters, &app.config.limits).http_status(StatusCode::BAD_REQUEST)?;

    if app.is_metric_hidden(&project.id, &entities, req.metric) {
        http_bail!(StatusCode::BAD_REQUEST, "Metric is hidden for this project")
    }
    if app.is_dimension_hidden(&project.id, &entities, req.dimension) {
        http_bail!(StatusCode::BAD_REQUEST, "Dimension is hidden for this project")
    }

    let max_results = app.config.limits.report_max_dimension_results;
    let stats = run_report(&app, move |conn| {
        reports::dimension_report(
            conn,
            &entities,
            "pageview",
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
