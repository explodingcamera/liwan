use crate::app::reports::{self, DateRange, Dimension, DimensionFilter, Metric, ReportStats};
use crate::app::Liwan;
use crate::utils::validate::{self, can_access_project};
use crate::web::session::SessionUser;
use crate::web::webext::{http_bail, ApiResult, PoemErrExt};

use poem::http::StatusCode;
use poem::web::Data;
use poem_openapi::param::Path;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};

#[derive(Object)]
struct GraphResponse {
    data: Vec<f64>,
}

#[derive(Object)]
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

pub struct DashboardAPI;

#[OpenApi]
impl DashboardAPI {
    #[oai(path = "/project/:project_id/earliest", method = "get")]
    async fn project_earliest_handler(
        &self,
        Path(project_id): Path<String>,
        Data(app): Data<&Liwan>,
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
        Data(app): Data<&Liwan>,
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
        let report = reports::overall_report(
            &conn,
            &entities,
            "pageview",
            &req.range,
            req.data_points,
            &req.filters,
            &req.metric,
        )
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(GraphResponse { data: report }))
    }

    #[oai(path = "/project/:project_id/stats", method = "post")]
    async fn project_stats_handler(
        &self,
        Json(req): Json<StatsRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&Liwan>,
        user: Option<SessionUser>,
    ) -> ApiResult<Json<StatsResponse>> {
        let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
        let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        if !can_access_project(&project, user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let stats = reports::overall_stats(&conn, &entities, "pageview", &req.range, &req.filters)
            .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let stats_prev = reports::overall_stats(&conn, &entities, "pageview", &req.range.prev(), &req.filters)
            .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let online = reports::online_users(&conn, &entities).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(StatsResponse { stats, stats_prev, current_visitors: online }))
    }

    #[oai(path = "/project/:project_id/dimension", method = "post")]
    async fn project_detailed_handler(
        &self,
        Json(req): Json<DimensionRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&Liwan>,
        user: Option<SessionUser>,
    ) -> ApiResult<Json<DimensionResponse>> {
        let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
        let entities = app.projects.entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        if !can_access_project(&project, user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let conn = app.events_conn().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let stats = reports::dimension_report(
            &conn,
            &entities,
            "pageview",
            &req.range,
            &req.dimension,
            &req.filters,
            &req.metric,
        )
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
