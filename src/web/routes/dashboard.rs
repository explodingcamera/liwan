use crate::app::models::{DisplayOverride, GeoDetail};
use crate::app::reports::{self, DateRange, Dimension, DimensionFilter, GraphInterval, Metric, ReportStats};
use crate::utils::validate::{self, can_access_project};
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

pub(super) fn metric_hidden(app: &crate::app::Liwan, project_id: &str, entities: &[String], metric: Metric) -> bool {
    match app
        .project_settings
        .get(project_id)
        .ok()
        .and_then(|settings| settings.metric_display_overrides.get(&metric.to_string()).copied())
        .unwrap_or(DisplayOverride::Auto)
    {
        DisplayOverride::Show => false,
        DisplayOverride::Hide => true,
        DisplayOverride::Auto => match metric {
            Metric::Views | Metric::UniqueVisitors => false,
            Metric::BounceRate | Metric::AvgTimeOnSite => {
                entities.iter().any(|entity_id| !app.settings.resolved_for_entity(entity_id).track_sessions)
            }
        },
    }
}

pub(super) fn dimension_hidden(
    app: &crate::app::Liwan,
    project_id: &str,
    entities: &[String],
    dimension: Dimension,
) -> bool {
    match app
        .project_settings
        .get(project_id)
        .ok()
        .and_then(|settings| settings.dimension_display_overrides.get(&dimension.to_string()).copied())
        .unwrap_or(DisplayOverride::Auto)
    {
        DisplayOverride::Show => false,
        DisplayOverride::Hide => true,
        DisplayOverride::Auto => match dimension {
            Dimension::UrlEntry | Dimension::UrlExit => {
                entities.iter().any(|entity_id| !app.settings.resolved_for_entity(entity_id).track_sessions)
            }
            Dimension::Country => entities
                .iter()
                .any(|entity_id| app.settings.resolved_for_entity(entity_id).track_geo == GeoDetail::None),
            Dimension::City => entities
                .iter()
                .any(|entity_id| app.settings.resolved_for_entity(entity_id).track_geo != GeoDetail::City),
            Dimension::UtmSource
            | Dimension::UtmMedium
            | Dimension::UtmCampaign
            | Dimension::UtmContent
            | Dimension::UtmTerm => {
                entities.iter().any(|entity_id| !app.settings.resolved_for_entity(entity_id).track_utm_params)
            }
            _ => false,
        },
    }
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
    MaybeAuth(user): MaybeAuth,
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
    MaybeAuth(user): MaybeAuth,
    Json(req): Json<GraphRequest>,
) -> ApiResult<Json<GraphResponse>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::IM_A_TEAPOT)?;
    let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    if !can_access_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    if metric_hidden(&app, &project.id, &entities, req.metric) {
        http_bail!(StatusCode::BAD_REQUEST, "Metric is hidden for this project")
    }

    let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let buckets = reports::build_graph_buckets(&req.range, req.interval, req.timezone.as_deref())
        .http_status(StatusCode::BAD_REQUEST)?;

    if buckets.len() > validate::MAX_DATAPOINTS as usize {
        http_bail!(StatusCode::BAD_REQUEST, "Too many data points")
    }

    let report = spawn_blocking(move || {
        reports::overall_report(&conn, &entities, "pageview", &req.range, &buckets, &req.filters, &req.metric)
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(GraphResponse { data: report }))
}

async fn project_stats_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    MaybeAuth(user): MaybeAuth,
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
        spawn_blocking(move || { reports::overall_stats(&conn, &entities, "pageview", &req.range, &req.filters) }),
        spawn_blocking(move || {
            reports::overall_stats(&conn2, &entities2, "pageview", &req2.range.prev(), &req2.filters)
        })
    )
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    let (mut stats, mut stats_prev) = (
        stats.http_status(StatusCode::INTERNAL_SERVER_ERROR)?,
        stats_prev.http_status(StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    if metric_hidden(&app, &project.id, &entities3, Metric::BounceRate) {
        stats.bounce_rate = None;
        stats_prev.bounce_rate = None;
    }
    if metric_hidden(&app, &project.id, &entities3, Metric::AvgTimeOnSite) {
        stats.avg_time_on_site = None;
        stats_prev.avg_time_on_site = None;
    }

    let online = reports::online_users(&app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?, &entities3)
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

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

    if !can_access_project(&project, user.as_ref()) {
        http_bail!(StatusCode::NOT_FOUND, "Project not found")
    }

    if metric_hidden(&app, &project.id, &entities, req.metric) {
        http_bail!(StatusCode::BAD_REQUEST, "Metric is hidden for this project")
    }
    if dimension_hidden(&app, &project.id, &entities, req.dimension) {
        http_bail!(StatusCode::BAD_REQUEST, "Dimension is hidden for this project")
    }

    let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = spawn_blocking(move || {
        reports::dimension_report(&conn, &entities, "pageview", &req.range, &req.dimension, &req.filters, &req.metric)
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
