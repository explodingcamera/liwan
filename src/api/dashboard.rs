use super::admin::{ProjectEntity, ProjectResponse, ProjectsResponse};
use super::session::SessionUser;
use super::webext::*;
use crate::app::models::{Project, UserRole};
use crate::app::reports::{self, DateRange, Metric};
use crate::app::App;
use crate::utils::validate;

use poem::http::StatusCode;
use poem::web::Data;
use poem_openapi::param::Path;
use poem_openapi::payload::{Json, Response};
use poem_openapi::{Object, OpenApi};

#[derive(Object)]
struct GraphResponse {
    data: Vec<u32>,
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
    total_views: u32,
    total_sessions: u32,
    unique_visitors: u32,
    avg_views_per_session: u32, // 3 decimal places
    current_visitors: u32,
}

pub(crate) struct DashboardAPI;

fn can_access_project(project: &Project, user: &Option<&SessionUser>) -> bool {
    project.public || user.map_or(false, |u| u.0.role == UserRole::Admin || u.0.projects.contains(&project.id))
}

#[OpenApi]
impl DashboardAPI {
    #[oai(path = "/projects", method = "get")]
    async fn projects_handler(
        &self,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> ApiResult<Response<Json<ProjectsResponse>>> {
        let projects = app.projects().http_err("Failed to get projects", StatusCode::INTERNAL_SERVER_ERROR)?;
        let projects: Vec<Project> = projects.into_iter().filter(|p| can_access_project(p, &user.as_ref())).collect();

        let mut resp = Vec::new();
        for project in projects {
            resp.push(ProjectResponse {
                id: project.id.clone(),
                display_name: project.display_name.clone(),
                entities: app
                    .project_entities(&project.id)
                    .http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?
                    .into_iter()
                    .map(|entity| ProjectEntity { id: entity.id, display_name: entity.display_name })
                    .collect(),
                public: project.public,
            });
        }

        Ok(Response::new(Json(ProjectsResponse { projects: resp })).header("Cache-Control", "private"))
    }

    #[oai(path = "/project/:project_id", method = "get")]
    async fn project_handler(
        &self,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> ApiResult<Response<Json<ProjectResponse>>> {
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if !can_access_project(&project, &user.as_ref()) {
            return Err(StatusCode::NOT_FOUND.into());
        }

        Ok(Response::new(Json(ProjectResponse {
            id: project.id.clone(),
            display_name: project.display_name.clone(),
            entities: app
                .project_entities(&project.id)
                .http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?
                .into_iter()
                .map(|entity| ProjectEntity { id: entity.id, display_name: entity.display_name })
                .collect(),
            public: project.public,
        }))
        .header("Cache-Control", "private"))
    }

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

        let online = reports::online_users(&conn, &entities).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(StatsResponse {
            total_views: stats.total_views,
            total_sessions: stats.total_sessions,
            unique_visitors: stats.unique_visitors,
            avg_views_per_session: stats.avg_views_per_session,
            current_visitors: online,
        }))
    }
}
