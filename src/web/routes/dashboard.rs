use std::sync::Arc;

use crate::app::Liwan;
use crate::app::reports::{self, DateRange, Dimension, DimensionFilter, Metric, ReportStats};
use crate::utils::validate::{self, can_access_project};
use crate::web::session::SessionUser;
use crate::web::webext::{ApiResult, PoemErrExt, http_bail};

use poem::http::StatusCode;
use poem::web::Data;
use poem_openapi::param::Path;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use tokio::task::spawn_blocking;

#[derive(Object)]
struct GraphResponse {
    data: Vec<f64>,
}

#[derive(Object, Clone)]
struct StatsRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct GraphRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
    data_points: u32,
    metric: Metric,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct StatsResponse {
    current_visitors: u64,
    stats: ReportStats,
    stats_prev: ReportStats,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct DimensionRequest {
    range: DateRange,
    filters: Vec<DimensionFilter>,
    metric: Metric,
    dimension: Dimension,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct DimensionResponse {
    data: Vec<DimensionTableRow>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct DimensionTableRow {
    dimension_value: String,
    value: f64,
    display_name: Option<String>,
    icon: Option<String>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct ConfigResponse {
    disable_favicons: bool,
}

pub struct DashboardAPI;

#[OpenApi]
impl DashboardAPI {
    #[oai(path = "/config", method = "get")]
    async fn config_handler(&self, Data(app): Data<&Arc<Liwan>>) -> ApiResult<Json<ConfigResponse>> {
        Ok(Json(ConfigResponse { disable_favicons: app.config.disable_favicons }))
    }

    #[oai(path = "/project/:project_id/earliest", method = "get")]
    async fn project_earliest_handler(
        &self,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        user: Option<SessionUser>,
    ) -> ApiResult<Json<Option<time::OffsetDateTime>>> {
        let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;

        if !can_access_project(&project, user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        let earliest = reports::earliest_timestamp(&conn, &entities).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(earliest))
    }

    #[oai(path = "/project/:project_id/graph", method = "post")]
    async fn project_graph_handler(
        &self,
        Json(req): Json<GraphRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        user: Option<SessionUser>,
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

    #[oai(path = "/project/:project_id/stats", method = "post")]
    async fn project_stats_handler(
        &self,
        Json(req): Json<StatsRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        user: Option<SessionUser>,
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

        let online =
            reports::online_users(&app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?, &entities3)
                .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(StatsResponse { stats, stats_prev, current_visitors: online }))
    }

    #[oai(path = "/project/:project_id/dimension", method = "post")]
    async fn project_detailed_handler(
        &self,
        Json(req): Json<DimensionRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        user: Option<SessionUser>,
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
}
