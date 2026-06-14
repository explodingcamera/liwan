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
    PASSWORD_MIN_LENGTH,
    app::{
        models::{
            CollectionSettings, Entity, EntityCollectionSettings, Project, ProjectDisplaySettings,
            ResolvedCollectionSettings, UserRole,
        },
        reports::{Dimension, Metric},
    },
    utils::validate::{can_enumerate_project, can_view_project},
    web::{
        RouterState,
        session::{Auth, MaybeAuth},
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
        .api_route("/project/{project_id}/settings", get(project_settings_handler))
        .api_route("/project/{project_id}/settings", put(project_settings_update_handler))
        .api_route("/projects", get(projects_handler))
        .api_route("/project/{project_id}", get(project_handler))
        .api_route("/project/{project_id}", delete(project_delete_handler))
        .api_route("/entities", get(entities_handler))
        .api_route("/entity", post(entity_create_handler))
        .api_route("/entity/{entity_id}", put(entity_update_handler))
        .api_route("/entity/{entity_id}/settings", get(entity_settings_handler))
        .api_route("/entity/{entity_id}/settings", put(entity_settings_update_handler))
        .api_route("/entity/{entity_id}", delete(entity_delete_handler))
        .api_route("/settings", get(settings_handler))
        .api_route("/settings", put(settings_update_handler))
        .api_route("/settings/prune", post(prune_handler))
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
    #[serde(default)]
    unlisted: bool,
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
    #[serde(default)]
    unlisted: bool,
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
    pub unlisted: bool,
    pub hidden_metrics: Vec<Metric>,
    pub hidden_dimensions: Vec<Dimension>,
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

impl ProjectResponse {
    fn new(app: &crate::app::Liwan, project: Project) -> anyhow::Result<Self> {
        let entities = app.projects.entities(&project.id)?;
        let entity_ids: Vec<String> = entities.iter().map(|entity| entity.id.clone()).collect();

        Ok(Self {
            id: project.id.clone(),
            display_name: project.display_name.clone(),
            entities: entities
                .into_iter()
                .map(|entity| ProjectEntity { id: entity.id, display_name: entity.display_name })
                .collect(),
            public: project.public,
            unlisted: project.unlisted,
            hidden_metrics: Metric::all()
                .iter()
                .copied()
                .filter(|metric| app.is_metric_hidden(&project.id, &entity_ids, *metric))
                .collect(),
            hidden_dimensions: Dimension::all()
                .iter()
                .copied()
                .filter(|dimension| app.is_dimension_hidden(&project.id, &entity_ids, *dimension))
                .collect(),
        })
    }
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
    unlisted: bool,
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

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct EntityCollectionSettingsResponse {
    settings: EntityCollectionSettings,
    resolved: ResolvedCollectionSettings,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct PruneRequest {
    dry_run: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct PruneEntityStats {
    entity_id: String,
    total_events: u64,
    deleted_events: u64,
    cleared_utm_events: u64,
    cleared_geo_events: u64,
    cleared_session_events: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct PruneResponse {
    dry_run: bool,
    entities: Vec<PruneEntityStats>,
    total: PruneEntityStats,
}

async fn get_users(
    app: State<RouterState>,
    Auth(user): Auth,
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
    Auth(session_user): Auth,
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
    Auth(session_user): Auth,
    params: Json<UpdatePasswordRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if session_user.role != UserRole::Admin || username != session_user.username {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if params.password.len() < PASSWORD_MIN_LENGTH {
        http_bail!(StatusCode::BAD_REQUEST, "password must be at least 8 characters long");
    }

    app.users
        .update_password(&username, &params.password)
        .http_err("Failed to update password", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn remove_user(
    app: State<RouterState>,
    Path(username): Path<String>,
    Auth(session_user): Auth,
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
    Auth(session_user): Auth,
    params: Json<CreateUserRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if session_user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if params.password.len() < PASSWORD_MIN_LENGTH {
        http_bail!(StatusCode::BAD_REQUEST, "password must be at least 8 characters long");
    }

    let app = app.app.clone();
    tokio::task::spawn_blocking(move || app.users.create(&params.username, &params.password, params.role, &[]))
        .await
        .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn project_create_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    Auth(user): Auth,
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
                unlisted: project.unlisted,
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
    Auth(user): Auth,
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
                unlisted: project.unlisted,
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
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectsResponse>>> {
    let projects = app.projects.all().http_err("Failed to get projects", StatusCode::INTERNAL_SERVER_ERROR)?;
    let projects: Vec<Project> = projects.into_iter().filter(|p| can_enumerate_project(p, user.as_ref())).collect();

    let mut resp = Vec::new();
    for project in projects {
        resp.push(
            ProjectResponse::new(&app, project).http_err("Failed to get project", StatusCode::INTERNAL_SERVER_ERROR)?,
        );
    }

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(ProjectsResponse { projects: resp })).into())
}

async fn project_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
    Path(project_id): Path<String>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectResponse>>> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    if !can_view_project(&project, user.as_ref()) {
        return Err(StatusCode::NOT_FOUND.into());
    }

    let resp =
        Json(ProjectResponse::new(&app, project).http_err("Failed to get project", StatusCode::INTERNAL_SERVER_ERROR)?);

    Ok(([(http::header::CACHE_CONTROL, "private")], resp).into())
}

async fn settings_handler(
    app: State<RouterState>,
    Auth(user): Auth,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<CollectionSettings>>> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(app.settings.global())).into())
}

async fn settings_update_handler(
    app: State<RouterState>,
    Auth(user): Auth,
    Json(settings): Json<CollectionSettings>,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.settings.update_global(&settings).http_err("Failed to update collection settings", StatusCode::BAD_REQUEST)?;

    Ok(empty_response())
}

async fn prune_handler(
    app: State<RouterState>,
    Auth(user): Auth,
    Json(req): Json<PruneRequest>,
) -> ApiResult<Json<PruneResponse>> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    let app = app.app.clone();
    let response = tokio::task::spawn_blocking(move || {
        let mut response = PruneResponse { dry_run: req.dry_run, ..Default::default() };
        for entity in app.entities.all()? {
            let settings = app.settings.resolved_for_entity(&entity.id);
            let stats = app.events.prune_entity(&entity.id, &settings, req.dry_run)?;
            let entity_stats = PruneEntityStats {
                entity_id: entity.id,
                total_events: stats.total_events,
                deleted_events: stats.deleted_events,
                cleared_utm_events: stats.cleared_utm_events,
                cleared_geo_events: stats.cleared_geo_events,
                cleared_session_events: stats.cleared_session_events,
            };
            response.total.total_events += entity_stats.total_events;
            response.total.deleted_events += entity_stats.deleted_events;
            response.total.cleared_utm_events += entity_stats.cleared_utm_events;
            response.total.cleared_geo_events += entity_stats.cleared_geo_events;
            response.total.cleared_session_events += entity_stats.cleared_session_events;
            response.entities.push(entity_stats);
        }
        anyhow::Ok(response)
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_err("Failed to prune collection data", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(response))
}

async fn project_settings_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectDisplaySettings>>> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    let settings = app
        .project_settings
        .get(&project_id)
        .http_err("Failed to get project display settings", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(settings)).into())
}

async fn project_settings_update_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    Auth(user): Auth,
    Json(mut settings): Json<ProjectDisplaySettings>,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
    settings.project_id = project_id;
    app.project_settings
        .update(&settings)
        .http_err("Failed to update project display settings", StatusCode::BAD_REQUEST)?;

    Ok(empty_response())
}

async fn entity_settings_handler(
    app: State<RouterState>,
    Path(entity_id): Path<String>,
    Auth(user): Auth,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<EntityCollectionSettingsResponse>>> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if !app.entities.exists(&entity_id).http_err("Failed to get entity", StatusCode::INTERNAL_SERVER_ERROR)? {
        http_bail!(StatusCode::NOT_FOUND, "Entity not found")
    }

    let settings = app.settings.entity(&entity_id);
    let resolved = app.settings.resolved_for_entity(&entity_id);
    Ok(([(http::header::CACHE_CONTROL, "private")], Json(EntityCollectionSettingsResponse { settings, resolved }))
        .into())
}

async fn entity_settings_update_handler(
    app: State<RouterState>,
    Path(entity_id): Path<String>,
    Auth(user): Auth,
    Json(mut settings): Json<EntityCollectionSettings>,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    if !app.entities.exists(&entity_id).http_err("Failed to get entity", StatusCode::INTERNAL_SERVER_ERROR)? {
        http_bail!(StatusCode::NOT_FOUND, "Entity not found")
    }

    settings.entity_id = entity_id;
    app.settings
        .update_entity(&settings)
        .http_err("Failed to update entity collection settings", StatusCode::BAD_REQUEST)?;

    Ok(empty_response())
}

async fn project_delete_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    Auth(user): Auth,
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
    Auth(user): Auth,
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
                    unlisted: project.unlisted,
                })
                .collect(),
        });
    }

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(EntitiesResponse { entities: resp })).into())
}

async fn entity_create_handler(
    app: State<RouterState>,
    Auth(user): Auth,
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
    Auth(user): Auth,
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
    Auth(user): Auth,
) -> ApiResult<impl IntoApiResponse> {
    if user.role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden")
    }

    app.entities.delete(&entity_id).http_err("Failed to delete entity", StatusCode::INTERNAL_SERVER_ERROR)?;
    app.settings.reload().http_err("Failed to reload collection settings", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}
