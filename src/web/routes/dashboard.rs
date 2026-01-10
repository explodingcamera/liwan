use crate::app::reports::{self, DateRange, Dimension, DimensionFilter, Metric, ReportStats};
use crate::utils::validate::{self, can_access_project};
use crate::web::session::SessionUser;
use crate::web::webext::{ApiResult, AxumErrExt, http_bail};
use crate::web::{MaybeExtract, RouterState};

use aide::axum::{ApiRouter, routing::*};
use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

pub fn router() -> ApiRouter<RouterState> {
    ApiRouter::new()
        .api_route("/config", get(config_handler))
        .api_route("/project/{project_id}/earliest", get(project_earliest_handler))
        .api_route("/project/{project_id}/graph", post(project_graph_handler))
        .api_route("/project/{project_id}/stats", post(project_stats_handler))
        .api_route("/project/{project_id}/dimension", post(project_detailed_handler))
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
struct GraphResponse {
    data: Vec<f64>,
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
    data_points: u32,
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
struct DimensionTableRow {
    dimension_value: String,
    value: f64,
    display_name: Option<String>,
    icon: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct ConfigResponse {
    disable_favicons: bool,
}

async fn config_handler(State(app): State<RouterState>) -> ApiResult<Json<ConfigResponse>> {
    Ok(Json(ConfigResponse { disable_favicons: app.config.disable_favicons }))
}

#[derive(Serialize, JsonSchema)]
struct EarliestResponse {
    earliest: Option<DateTime<Utc>>,
}

async fn project_earliest_handler(
    app: State<RouterState>,
    MaybeExtract(user): MaybeExtract<SessionUser>,
    Path(project_id): Path<String>,
) -> ApiResult<Json<EarliestResponse>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;

    if !can_access_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let earliest = reports::earliest_timestamp(&conn, &entities).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(EarliestResponse { earliest }))
}

async fn project_graph_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeExtract(user): MaybeExtract<SessionUser>,
    Json(req): Json<GraphRequest>,
) -> ApiResult<Json<GraphResponse>> {
    if req.data_points > validate::MAX_DATAPOINTS {
        http_bail!(StatusCode::BAD_REQUEST, "Too many data points")
    }

    let project = app.projects.get(&project_id).http_status(StatusCode::IM_A_TEAPOT)?;
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    if !can_access_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    let report = spawn_blocking(move || {
        reports::overall_report_cached(
            &conn,
            &entities,
            "pageview",
            &req.range,
            req.data_points,
            &req.filters,
            &req.metric,
        )
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(GraphResponse { data: report }))
}

async fn project_stats_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeExtract(user): MaybeExtract<SessionUser>,
    Json(req): Json<StatsRequest>,
) -> ApiResult<Json<StatsResponse>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    if !can_access_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let (entities2, entities3) = (entities.clone(), entities.clone());

    let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    let req2 = req.clone();
    let conn2 = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    let (stats, stats_prev) = tokio::try_join!(
        spawn_blocking(move || {
            reports::overall_stats_cached(&conn, &entities, "pageview", &req.range, &req.filters)
        }),
        spawn_blocking(move || {
            reports::overall_stats_cached(&conn2, &entities2, "pageview", &req2.range.prev(), &req2.filters)
        })
    )
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    let (stats, stats_prev) = (
        stats.http_status(StatusCode::INTERNAL_SERVER_ERROR)?,
        stats_prev.http_status(StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let online = reports::online_users(&app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?, &entities3)
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(StatsResponse { current_visitors: online, stats, stats_prev }))
}

async fn project_detailed_handler(
    app: State<RouterState>,
    MaybeExtract(user): MaybeExtract<SessionUser>,
    Path(project_id): Path<String>,
    Json(req): Json<DimensionRequest>,
) -> ApiResult<Json<DimensionResponse>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    if !can_access_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = spawn_blocking(move || {
        reports::dimension_report_cached(
            &conn,
            &entities,
            "pageview",
            &req.range,
            &req.dimension,
            &req.filters,
            &req.metric,
        )
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

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
            _ => {
                data.push(DimensionTableRow { dimension_value: key, value, display_name: None, icon: None });
            }
        }
    }

    Ok(Json(DimensionResponse { data }))
}
