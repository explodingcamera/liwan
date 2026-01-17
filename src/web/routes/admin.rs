use aide::{
    UseApi,
    axum::{ApiRouter, IntoApiResponse, routing::*},
};
use axum::{
    Json,
    extract::{Path, State},
};
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    app::models::{Entity, Project, UserRole},
    utils::validate::can_access_project,
    web::{
        MaybeExtract, RouterState, SessionUser,
        webext::{ApiResult, AxumErrExt, empty_response, http_bail},
    },
};

pub fn router() -> ApiRouter<RouterState> {
    ApiRouter::new()
        .api_route("/users", get(get_users))
        .api_route("/user/{username}", put(update_user))
        .api_route("/user/{username}/password", put(update_user_password))
        .api_route("/user/{username}", delete(remove_user))
        .api_route("/user", post(create_user))
        .api_route("/project/{project_id}", post(project_create_handler))
        .api_route("/project/{project_id}", put(project_update_handler))
        .api_route("/projects", get(projects_handler))
        .api_route("/project/{project_id}", get(project_handler))
        .api_route("/project/{project_id}", delete(project_delete_handler))
        .api_route("/entities", get(entities_handler))
        .api_route("/entity", post(entity_create_handler))
        .api_route("/entity/{entity_id}", put(entity_update_handler))
        .api_route("/entity/{entity_id}", delete(entity_delete_handler))
}

pub struct AdminAPI;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct CreateUserRequest {
    username: String,
    password: String,
    role: UserRole,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct UpdateUserRequest {
    role: UserRole,
    projects: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct UserResponse {
    username: String,
    role: UserRole,
    projects: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct UsersResponse {
    users: Vec<UserResponse>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct UpdateProjectRequest {
    project: Option<UpdateProjectInfo>,
    entities: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct UpdateProjectInfo {
    display_name: String,
    public: bool,
    secret: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct UpdatePasswordRequest {
    password: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CreateProjectRequest {
    display_name: String,
    public: bool,
    secret: Option<String>,
    entities: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: String,
    pub display_name: String,
    pub entities: Vec<ProjectEntity>,
    pub public: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ProjectsResponse {
    pub projects: Vec<ProjectResponse>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntity {
    pub id: String,
    pub display_name: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct EntityResponse {
    id: String,
    display_name: String,
    projects: Vec<EntityProject>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct EntityProject {
    id: String,
    display_name: String,
    public: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CreateEntityRequest {
    id: String,
    display_name: String,
    projects: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct UpdateEntityRequest {
    display_name: Option<String>,
    projects: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct EntitiesResponse {
    entities: Vec<EntityResponse>,
}

async fn get_users(
    app: State<RouterState>,
    SessionUser(user): SessionUser,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<UsersResponse>>> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    let users = app
        .users
        .all()
        .http_err("Failed to get users", StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|u| UserResponse { username: u.username.clone(), role: u.role, projects: u.projects.clone() })
        .collect();

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(UsersResponse { users })).into())
}

async fn update_user(
    app: State<RouterState>,
    Path(username): Path<String>,
    SessionUser(session_user): SessionUser,
    user: Json<UpdateUserRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if session_user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if username == session_user.username && user.role != session_user.role {
        http_bail!(StatusCode::FORBIDDEN, "Cannot change own role")
    }

    app.users
        .update(&username, user.role, user.projects.as_slice())
        .http_err("Failed to update user", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn update_user_password(
    app: State<RouterState>,
    Path(username): Path<String>,
    SessionUser(session_user): SessionUser,
    password: Json<UpdatePasswordRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if session_user.role != UserRole::Admin || username != session_user.username {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.users
        .update_password(&username, &password.password)
        .http_err("Failed to update password", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn remove_user(
    app: State<RouterState>,
    Path(username): Path<String>,
    SessionUser(session_user): SessionUser,
) -> ApiResult<impl IntoApiResponse> {
    if session_user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if username == session_user.username {
        http_bail!(StatusCode::FORBIDDEN, "Cannot delete own user")
    }

    app.users.delete(&username).http_err("Failed to delete user", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn create_user(
    app: State<RouterState>,
    SessionUser(session_user): SessionUser,
    user: Json<CreateUserRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if session_user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    let app = app.clone();
    tokio::task::spawn_blocking(move || app.users.create(&user.username, &user.password, user.role, &[]))
        .await
        .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn project_create_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    SessionUser(user): SessionUser,
    Json(project): Json<CreateProjectRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.projects
        .create(
            &Project {
                id: project_id,
                display_name: project.display_name,
                public: project.public,
                secret: project.secret,
            },
            project.entities.as_slice(),
        )
        .http_err("Failed to create project", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn project_update_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    SessionUser(user): SessionUser,
    Json(req): Json<UpdateProjectRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if let Some(project) = req.project {
        app.projects
            .update(&Project {
                id: project_id.clone(),
                display_name: project.display_name,
                public: project.public,
                secret: project.secret,
            })
            .http_err("Failed to update project", StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(entities) = req.entities {
        app.projects
            .update_entities(&project_id, entities.as_slice())
            .http_err("Failed to update project entities", StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(empty_response())
}

async fn projects_handler(
    app: State<RouterState>,
    MaybeExtract(user): MaybeExtract<SessionUser>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectsResponse>>> {
    let projects = app.projects.all().http_err("Failed to get projects", StatusCode::INTERNAL_SERVER_ERROR)?;
    let projects: Vec<Project> = projects.into_iter().filter(|p| can_access_project(p, user.as_ref())).collect();

    let mut resp = Vec::new();
    for project in projects {
        resp.push(ProjectResponse {
            id: project.id.clone(),
            display_name: project.display_name.clone(),
            entities: app
                .projects
                .entities(&project.id)
                .http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?
                .into_iter()
                .map(|entity| ProjectEntity { id: entity.id, display_name: entity.display_name })
                .collect(),
            public: project.public,
        });
    }

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(ProjectsResponse { projects: resp })).into())
}

async fn project_handler(
    app: State<RouterState>,
    MaybeExtract(user): MaybeExtract<SessionUser>,
    Path(project_id): Path<String>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectResponse>>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    if !can_access_project(&project, user.as_ref()) {
        return Err(StatusCode::NOT_FOUND.into());
    }

    let resp = Json(ProjectResponse {
        id: project.id.clone(),
        display_name: project.display_name.clone(),
        entities: app
            .projects
            .entities(&project.id)
            .http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter()
            .map(|entity| ProjectEntity { id: entity.id, display_name: entity.display_name })
            .collect(),
        public: project.public,
    });

    Ok(([(http::header::CACHE_CONTROL, "private")], resp).into())
}

async fn project_delete_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    SessionUser(user): SessionUser,
) -> ApiResult<impl IntoApiResponse> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.projects.delete(&project.id).http_err("Failed to delete project", StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(empty_response())
}

async fn entities_handler(
    app: State<RouterState>,
    SessionUser(user): SessionUser,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<EntitiesResponse>>> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    let entities = app.entities.all().http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut resp = Vec::new();
    for entity in entities {
        resp.push(EntityResponse {
            id: entity.id.clone(),
            display_name: entity.display_name.clone(),
            projects: app
                .entities
                .projects(&entity.id)
                .http_err("Failed to get projects", StatusCode::INTERNAL_SERVER_ERROR)?
                .into_iter()
                .map(|project| EntityProject {
                    id: project.id,
                    display_name: project.display_name,
                    public: project.public,
                })
                .collect(),
        });
    }

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(EntitiesResponse { entities: resp })).into())
}

async fn entity_create_handler(
    app: State<RouterState>,
    SessionUser(user): SessionUser,
    Json(entity): Json<CreateEntityRequest>,
) -> ApiResult<Json<EntityResponse>> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.entities
        .create(
            &Entity { id: entity.id.clone(), display_name: entity.display_name.clone() },
            entity.projects.as_slice(),
        )
        .http_err("Failed to create entity", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(EntityResponse { id: entity.id, display_name: entity.display_name, projects: Vec::new() }))
}

async fn entity_update_handler(
    app: State<RouterState>,
    Path(entity_id): Path<String>,
    SessionUser(user): SessionUser,
    Json(entity): Json<UpdateEntityRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if let Some(display_name) = entity.display_name {
        app.entities
            .update(&Entity { id: entity_id.clone(), display_name })
            .http_err("Failed to update entity", StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(projects) = entity.projects {
        app.entities
            .update_projects(&entity_id, projects.as_slice())
            .http_err("Failed to update entity projects", StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(empty_response())
}

async fn entity_delete_handler(
    app: State<RouterState>,
    Path(entity_id): Path<String>,
    SessionUser(user): SessionUser,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.entities.delete(&entity_id).http_err("Failed to delete entity", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}
