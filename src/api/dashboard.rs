use super::session::SessionUser;
use super::webext::*;
use crate::app::models::{Entity, Project, User, UserRole};
use crate::app::reports::{self, DateRange, Metric, ReportStats};
use crate::app::App;
use crate::utils::validate;

use poem::http::StatusCode;
use poem::web::{Data, Path};
use poem_openapi::payload::{Json, Response};
use poem_openapi::{Object, OpenApi};
use std::collections::BTreeMap;

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
struct ProjectResponse {
    display_name: String,
    entities: BTreeMap<String, String>,
    public: bool,
}

#[derive(Object)]
struct ProjectsResponse {
    projects: BTreeMap<String, ProjectResponse>,
}

#[derive(Object)]
struct GraphResponse {
    data: Vec<u32>,
}

#[derive(Debug, Object, Clone)]
pub(crate) struct CreateUserRequest {
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) role: UserRole,
}

pub(crate) struct DashboardAPI;

fn can_access_project(project: &Project, user: &Option<&SessionUser>) -> bool {
    project.public || user.map_or(false, |u| u.0.role == UserRole::Admin || u.0.projects.contains(&project.id))
}

#[OpenApi]
impl DashboardAPI {
    #[oai(path = "/users", method = "get")]
    async fn users_handler(
        &self,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Response<Json<Vec<User>>>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }
        let users = app.users().http_err("Failed to get users", StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Response::new(Json(users)).header("Cache-Control", "private"))
    }

    #[oai(path = "/user/:username", method = "put")]
    async fn user_update_handler(
        &self,
        Path(username): Path<String>,
        Json(user): Json<User>,
        Data(app): Data<&App>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<Json<User>> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        if username == session_user.username && user.role != session_user.role {
            http_bail!(StatusCode::FORBIDDEN, "Cannot change own role")
        }

        let user = app.user_update(&user).http_err("Failed to update user", StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(user))
    }

    #[oai(path = "/user/:username/password", method = "put")]
    async fn user_password_handler(
        &self,
        Path(username): Path<String>,
        Json(password): Json<String>,
        Data(app): Data<&App>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.user_update_password(&username, &password)
            .http_err("Failed to update password", StatusCode::INTERNAL_SERVER_ERROR)?;
        EmptyResponse::ok()
    }

    #[oai(path = "/user/:username", method = "delete")]
    async fn user_delete_handler(
        &self,
        Path(username): Path<String>,
        Data(app): Data<&App>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        if username == session_user.username {
            http_bail!(StatusCode::FORBIDDEN, "Cannot delete own user")
        }

        app.user_delete(&username).http_err("Failed to delete user", StatusCode::INTERNAL_SERVER_ERROR)?;
        EmptyResponse::ok()
    }

    #[oai(path = "/user", method = "post")]
    async fn user_create_handler(
        &self,
        Json(user): Json<CreateUserRequest>,
        Data(app): Data<&App>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.user_create(&user.username, &user.password, user.role, &[])
            .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/entities", method = "get")]
    async fn entities_handler(
        &self,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Response<Json<BTreeMap<String, String>>>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }
        let entities = app.entities().http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Response::new(Json(entities)).header("Cache-Control", "private"))
    }

    #[oai(path = "/entity", method = "post")]
    async fn entity_create_handler(
        &self,
        Json(entity): Json<Entity>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Json<Entity>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        let entity =
            app.entity_create(&entity).http_err("Failed to create entity", StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(entity))
    }

    #[oai(path = "/entity/:entity_id", method = "delete")]
    async fn entity_delete_handler(
        &self,
        Path(entity_id): Path<String>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.entity_delete(&entity_id).http_err("Failed to delete entity", StatusCode::INTERNAL_SERVER_ERROR)?;
        EmptyResponse::ok()
    }

    #[oai(path = "/projects", method = "get")]
    async fn projects_handler(
        &self,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> ApiResult<Response<Json<ProjectsResponse>>> {
        let projects = app.projects().http_err("Failed to get projects", StatusCode::INTERNAL_SERVER_ERROR)?;
        let projects: Vec<Project> = projects.into_iter().filter(|p| can_access_project(p, &user.as_ref())).collect();

        let mut resp = BTreeMap::new();
        for project in projects {
            resp.insert(
                project.id.clone(),
                ProjectResponse {
                    display_name: project.display_name.clone(),
                    entities: app
                        .project_entities(&project.id)
                        .http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?,
                    public: project.public,
                },
            );
        }

        Ok(Response::new(Json(ProjectsResponse { projects: resp })).header("Cache-Control", "private"))
    }

    #[oai(path = "/project", method = "post")]
    async fn project_create_handler(
        &self,
        Json(project): Json<Project>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Json<Project>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        let project =
            app.project_create(&project).http_err("Failed to create project", StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(project))
    }

    #[oai(path = "/project", method = "put")]
    async fn project_update_handler(
        &self,
        Json(project): Json<Project>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Json<Project>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }
        let project =
            app.project_update(&project).http_err("Failed to update project", StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(project))
    }

    #[oai(path = "/project/:project_id/entities", method = "get")]
    async fn project_entities_handler(
        &self,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        user: Option<SessionUser>,
    ) -> ApiResult<Response<Json<BTreeMap<String, String>>>> {
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        Ok(Response::new(Json(
            app.project_entities(&project_id).http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?,
        ))
        .header("Cache-Control", "private"))
    }

    #[oai(path = "/project/:project_id/entity/:entity_id", method = "put")]
    async fn project_entity_update_handler(
        &self,
        Path(project_id): Path<String>,
        Path(entity_id): Path<String>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.project_add_entity(&project.id, &entity_id)
            .http_err("Failed to add entity", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/project/:project_id/entity/:entity_id", method = "delete")]
    async fn project_entity_delete_handler(
        &self,
        Path(project_id): Path<String>,
        Path(entity_id): Path<String>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.project_remove_entity(&project.id, &entity_id)
            .http_err("Failed to remove entity", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/project/:project_id", method = "delete")]
    async fn project_delete_handler(
        &self,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.project_delete(&project.id).http_err("Failed to delete project", StatusCode::INTERNAL_SERVER_ERROR)?;
        EmptyResponse::ok()
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

        let conn = app.conn_events().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        let entities = app.project_entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

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
    ) -> ApiResult<Json<ReportStats>> {
        let conn = app.conn_events().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        let project = app.project(&project_id).http_status(StatusCode::NOT_FOUND)?;
        let entities = app.project_entity_ids(&project.id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        if !can_access_project(&project, &user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }

        let stats = reports::overall_stats(&conn, &entities, "pageview", &req.range, &[])
            .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(stats))
    }
}
