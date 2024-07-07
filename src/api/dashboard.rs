use super::session::SessionUser;
use super::webext::*;
use crate::app::models::{Project, UserRole};
use crate::app::App;
use crate::reports::{self, DateRange, Metric, ReportStats};
use crate::utils::validate;

use poem::http::StatusCode;
use poem::web::{Data, Path};
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use std::collections::BTreeMap;

#[derive(Object)]
pub struct StatsRequest {
    pub range: DateRange,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
pub struct GraphRequest {
    pub range: DateRange,
    pub data_points: u32,
    pub metric: Metric,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct ProjectResponse {
    pub display_name: String,
    pub entities: BTreeMap<String, String>,
    pub public: bool,
}

#[derive(Object)]
struct ProjectsResponse {
    projects: BTreeMap<String, ProjectResponse>,
}

#[derive(Object)]
struct GraphResponse {
    pub data: Vec<u32>,
}

pub struct DashboardAPI;

pub fn can_access_project(project: &Project, user: &Option<&SessionUser>) -> bool {
    project.public || user.map_or(false, |u| u.0.role == UserRole::Admin || u.0.projects.contains(&project.id))
}

#[OpenApi]
impl DashboardAPI {
    #[oai(path = "/projects", method = "get")]
    async fn projects_handler(
        &self,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> poem::Result<Json<ProjectsResponse>> {
        let projects = app.projects().http_internal("Failed to get projects")?;
        let projects: Vec<Project> = projects.into_iter().filter(|p| can_access_project(p, &user.as_ref())).collect();

        let mut resp = BTreeMap::new();
        for project in projects {
            resp.insert(
                project.id.clone(),
                ProjectResponse {
                    display_name: project.display_name.clone(),
                    entities: app.project_entities(&project.id).http_internal("Failed to get entity names")?,
                    public: project.public,
                },
            );
        }

        Ok(Json(ProjectsResponse { projects: resp }))
    }

    #[oai(path = "/project/:project_id/graph", method = "post")]
    async fn project_graph_handler(
        &self,
        Json(req): Json<GraphRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> APIResult<Json<GraphResponse>> {
        if req.data_points > validate::MAX_DATAPOINTS {
            http_bail!(StatusCode::BAD_REQUEST, "Too many data points")
        }

        let conn = app.conn().http_internal("Failed to get connection")?;
        let project = app.project(&project_id).http_not_found("Project not found")?;
        let entities = app.project_entity_ids(&project.id).http_internal("Failed to get entity names")?;

        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let report =
            reports::overall_report(&conn, &entities, "pageview", &req.range, req.data_points, &[], &req.metric)
                .http_internal("Failed to generate report")?;

        Ok(Json(GraphResponse { data: report }))
    }

    #[oai(path = "/project/:project_id/stats", method = "post")]
    async fn project_stats_handler(
        &self,
        Json(req): Json<StatsRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> APIResult<Json<ReportStats>> {
        let conn = app.conn().http_internal("Failed to get connection")?;
        let project = app.project(&project_id).http_not_found("Project not found")?;
        let entities = app.project_entity_ids(&project.id).http_internal("Failed to get entity names")?;

        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let stats = reports::overall_stats(&conn, &entities, "pageview", &req.range, &[])
            .http_internal("Failed to generate stats")?;

        Ok(Json(stats))
    }
}
