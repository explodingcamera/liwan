use super::{webext::*, SessionUser};
use crate::app::reports::{self, DateRange, Dimension, Metric, ReportStats};
use crate::app::App;
use crate::utils::validate::{self, can_access_project};

use poem::http::StatusCode;
use poem::web::Data;
use poem_openapi::param::Path;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};

#[derive(Object)]
struct GraphResponse {
    data: Vec<u64>,
}

#[derive(Object)]
struct StatsRequest {
    range: DateRange,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct GraphRequest {
    range: DateRange,
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
    value: u64,
    display_name: Option<String>,
    icon: Option<String>,
}

pub(crate) struct DashboardAPI;

#[OpenApi]
impl DashboardAPI {
    #[oai(path = "/project/:project_id/graph", method = "post")]
    async fn project_graph_handler(
        &self,
        Json(req): Json<GraphRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> ApiResult<Json<GraphResponse>> {
        if req.data_points > validate::MAX_DATAPOINTS {
            http_bail!(StatusCode::BAD_REQUEST, "Too many data points")
        }

        let project = app.project(&project_id).http_status(StatusCode::IM_A_TEAPOT)?;
        let entities = app.project_entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let conn = app.conn_events().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        let report =
            reports::overall_report(&conn, &entities, "pageview", &req.range, req.data_points, &[], &req.metric)
                .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(GraphResponse { data: report }))
    }

    #[oai(path = "/project/:project_id/stats", method = "post")]
    async fn project_stats_handler(
        &self,
        Json(req): Json<StatsRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> ApiResult<Json<StatsResponse>> {
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        let entities = app.project_entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let conn = app.conn_events().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let stats = reports::overall_stats(&conn, &entities, "pageview", &req.range, &[])
            .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let stats_prev = reports::overall_stats(&conn, &entities, "pageview", &req.range.prev(), &[])
            .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let online = reports::online_users(&conn, &entities).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(StatsResponse { stats, stats_prev, current_visitors: online }))
    }

    #[oai(path = "/project/:project_id/dimension", method = "post")]
    async fn project_detailed_handler(
        &self,
        Json(req): Json<DimensionRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> ApiResult<Json<DimensionResponse>> {
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        let entities = app.project_entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let conn = app.conn_events().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let stats =
            reports::dimension_report(&conn, &entities, "pageview", &req.range, &req.dimension, &[], &req.metric)
                .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut data = Vec::new();
        for (key, value) in stats {
            match req.dimension {
                Dimension::Referrer => {
                    let icon = crate::utils::referrer::get_referer_icon(&key);
                    let display_name = crate::utils::referrer::get_referer_name(&key);
                    data.push(DimensionTableRow { dimension_value: key, value, display_name, icon });
                }
                Dimension::Browser => {
                    let display_name = match key.as_str() {
                        "Edge" => Some("Microsoft Edge".to_string()),
                        _ => None,
                    };

                    data.push(DimensionTableRow { dimension_value: key, value, display_name, icon: None });
                }
                _ => {
                    data.push(DimensionTableRow { dimension_value: key, value, display_name: None, icon: None });
                }
            }
        }

        Ok(Json(DimensionResponse { data }))
    }
}
