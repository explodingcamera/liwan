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
            AccessPermission, AccessScope, ApiKeyExpiration, CollectionSettings, Entity, EntityCollectionSettings,
            Project, ProjectDisplaySettings, ProjectVisibility, ResolvedCollectionSettings, UserRole,
        },
        reports::{Dimension, Metric},
    },
    utils::validate::{can_enumerate_project, can_view_project},
    web::{
        RouterState,
        session::{Admin, MaybeAuth},
        webext::{ApiResult, AxumErrExt, empty_response, http_bail},
    },
};

pub fn router() -> ApiRouter<RouterState> {
    ApiRouter::new()
        .api_route("/users", get(get_users))
        .api_route("/user/{username}", put(update_user))
        .api_route("/user/{username}/password", put(update_user_password))
        .api_route("/user/{username}/sessions", delete(revoke_user_sessions))
        .api_route("/user/{username}", delete(remove_user))
        .api_route("/user", post(create_user))
        .api_route("/teams", get(teams_handler))
        .api_route("/teams", post(team_create_handler))
        .api_route("/team/{team_id}", put(team_update_handler))
        .api_route("/team/{team_id}", delete(team_delete_handler))
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
        .api_route("/api-keys", get(api_keys_handler))
        .api_route("/api-keys", post(api_key_create_handler))
        .api_route("/api-keys/{key_id}", put(api_key_update_handler))
        .api_route("/api-keys/{key_id}", delete(api_key_delete_handler))
        .api_route("/api-keys/{key_id}/regenerate", post(api_key_regenerate_handler))
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
    teams: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct UserResponse {
    username: String,
    role: UserRole,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct UsersResponse {
    users: Vec<UserResponse>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TeamResponse {
    id: String,
    display_name: String,
    users: Vec<String>,
    entities: AccessScope,
    projects: AccessScope,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct TeamsResponse {
    teams: Vec<TeamResponse>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct CreateTeamRequest {
    display_name: String,
}

#[derive(Serialize, JsonSchema)]
struct CreateTeamResponse {
    id: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpdateTeamRequest {
    display_name: String,
    users: Vec<String>,
    projects: AccessScope,
    entities: Option<AccessScope>,
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
    visibility: ProjectVisibility,
    secret: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct UpdatePasswordRequest {
    password: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CreateProjectRequest {
    display_name: String,
    visibility: ProjectVisibility,
    secret: Option<String>,
    entities: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: String,
    pub display_name: String,
    pub entities: Vec<ProjectEntity>,
    pub visibility: ProjectVisibility,
    pub hidden_metrics: Vec<Metric>,
    pub hidden_dimensions: Vec<Dimension>,
    pub custom_events_display: crate::app::models::DisplayOverride,
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
        let collection = app.settings.resolved_for_entities(&entity_ids);
        let display = app.project_settings.get(&project.id)?;

        Ok(Self {
            id: project.id.clone(),
            display_name: project.display_name.clone(),
            entities: entities
                .into_iter()
                .map(|entity| ProjectEntity { id: entity.id, display_name: entity.display_name })
                .collect(),
            visibility: project.visibility,
            custom_events_display: display.custom_events_display(),
            hidden_metrics: Metric::all()
                .iter()
                .copied()
                .filter(|metric| display.is_metric_hidden(&collection, *metric))
                .collect(),
            hidden_dimensions: Dimension::all()
                .iter()
                .copied()
                .filter(|dimension| display.is_dimension_hidden(&collection, *dimension))
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
    visibility: ProjectVisibility,
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
struct ApiKeyResponse {
    id: String,
    display_name: String,
    entities: AccessScope,
    projects: AccessScope,
    permissions: Vec<AccessPermission>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct ApiKeysResponse {
    keys: Vec<ApiKeyResponse>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CreateApiKeyRequest {
    display_name: String,
    entities: AccessScope,
    projects: AccessScope,
    permissions: Vec<AccessPermission>,
    expiration: ApiKeyExpiration,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CreateApiKeyResponse {
    key: ApiKeyResponse,
    plaintext: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct UpdateApiKeyRequest {
    display_name: String,
    entities: AccessScope,
    projects: AccessScope,
    permissions: Vec<AccessPermission>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
struct RegenerateApiKeyRequest {
    expiration: ApiKeyExpiration,
}

impl From<crate::app::models::ApiKey> for ApiKeyResponse {
    fn from(key: crate::app::models::ApiKey) -> Self {
        Self {
            id: key.id,
            display_name: key.display_name,
            entities: key.entities,
            projects: key.projects,
            permissions: key.permissions,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
        }
    }
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
    Admin(_): Admin,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<UsersResponse>>> {
    let users = tokio::task::spawn_blocking(move || app.users.all())
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to get users", StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|u| UserResponse { username: u.username, role: u.role })
        .collect();

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(UsersResponse { users })).into())
}

async fn teams_handler(
    app: State<RouterState>,
    Admin(_): Admin,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<TeamsResponse>>> {
    let teams = tokio::task::spawn_blocking(move || app.teams.all())
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to get teams", StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(|team| TeamResponse {
            id: team.id,
            display_name: team.display_name,
            users: team.users,
            entities: team.access.entities,
            projects: team.access.projects,
        })
        .collect();
    Ok(([(http::header::CACHE_CONTROL, "private")], Json(TeamsResponse { teams })).into())
}

async fn team_create_handler(
    app: State<RouterState>,
    Admin(_): Admin,
    Json(req): Json<CreateTeamRequest>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<CreateTeamResponse>>> {
    let id = tokio::task::spawn_blocking(move || app.teams.create(&req.display_name))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to create team", StatusCode::BAD_REQUEST)?;
    Ok(Json(CreateTeamResponse { id }).into())
}

async fn team_update_handler(
    app: State<RouterState>,
    Path(team_id): Path<String>,
    Admin(_): Admin,
    Json(req): Json<UpdateTeamRequest>,
) -> ApiResult<impl IntoApiResponse> {
    let updated = tokio::task::spawn_blocking(move || {
        app.teams.update(&team_id, &req.display_name, &req.users, &req.projects, req.entities.as_ref())
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_err("Failed to update team", StatusCode::BAD_REQUEST)?;
    if !updated {
        http_bail!(StatusCode::NOT_FOUND, "Team not found");
    }
    Ok(empty_response())
}

async fn team_delete_handler(
    app: State<RouterState>,
    Path(team_id): Path<String>,
    Admin(_): Admin,
) -> ApiResult<impl IntoApiResponse> {
    tokio::task::spawn_blocking(move || app.teams.delete(&team_id))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to delete team", StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(empty_response())
}

async fn update_user(
    app: State<RouterState>,
    Path(username): Path<String>,
    Admin(session_user): Admin,
    user: Json<UpdateUserRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if username.eq_ignore_ascii_case(&session_user.username) && user.role != session_user.role {
        http_bail!(StatusCode::FORBIDDEN, "Cannot change own role")
    }

    let updated = tokio::task::spawn_blocking(move || app.users.update(&username, user.role, &user.teams))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to update user", StatusCode::BAD_REQUEST)?;
    if !updated {
        http_bail!(StatusCode::NOT_FOUND, "User not found")
    }

    Ok(empty_response())
}

async fn update_user_password(
    app: State<RouterState>,
    Path(username): Path<String>,
    Admin(session_user): Admin,
    params: Json<UpdatePasswordRequest>,
) -> ApiResult<impl IntoApiResponse> {
    let username = username.to_lowercase();
    if username == session_user.username {
        http_bail!(StatusCode::BAD_REQUEST, "Use your account settings to change your own password")
    }

    if params.password.len() < PASSWORD_MIN_LENGTH {
        http_bail!(StatusCode::BAD_REQUEST, "password must be at least 8 characters long");
    }

    let users = app.users.clone();
    let changed = tokio::task::spawn_blocking(move || users.update_password(&username, &params.password, None, None))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to update password", StatusCode::INTERNAL_SERVER_ERROR)?;
    if !changed {
        http_bail!(StatusCode::BAD_REQUEST, "Failed to reset password");
    }

    Ok(empty_response())
}

async fn revoke_user_sessions(
    app: State<RouterState>,
    Path(username): Path<String>,
    Admin(_): Admin,
) -> ApiResult<impl IntoApiResponse> {
    let sessions = app.sessions.clone();
    tokio::task::spawn_blocking(move || sessions.revoke_user(&username))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to revoke sessions", StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(empty_response())
}

async fn remove_user(
    app: State<RouterState>,
    Path(username): Path<String>,
    Admin(session_user): Admin,
) -> ApiResult<impl IntoApiResponse> {
    if username.eq_ignore_ascii_case(&session_user.username) {
        http_bail!(StatusCode::FORBIDDEN, "Cannot delete own user")
    }

    tokio::task::spawn_blocking(move || app.users.delete(&username))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to delete user", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn create_user(
    app: State<RouterState>,
    Admin(_): Admin,
    params: Json<CreateUserRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if params.password.len() < PASSWORD_MIN_LENGTH {
        http_bail!(StatusCode::BAD_REQUEST, "password must be at least 8 characters long");
    }

    let app = app.app.clone();
    tokio::task::spawn_blocking(move || app.users.create(&params.username, &params.password, params.role))
        .await
        .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}

async fn project_create_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    Admin(_): Admin,
    Json(project): Json<CreateProjectRequest>,
) -> ApiResult<impl IntoApiResponse> {
    app.projects
        .create(
            &Project {
                id: project_id,
                display_name: project.display_name,
                visibility: project.visibility,
                secret: project.secret,
            },
            project.entities.as_slice(),
        )
        .http_err("Failed to create project", StatusCode::BAD_REQUEST)?;

    Ok(empty_response())
}

async fn project_update_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    Admin(_): Admin,
    Json(req): Json<UpdateProjectRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if let Some(project) = req.project {
        app.projects
            .update(&Project {
                id: project_id.clone(),
                display_name: project.display_name,
                visibility: project.visibility,
                secret: project.secret,
            })
            .http_err("Failed to update project", StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(entities) = req.entities {
        app.projects
            .update_entities(&project_id, entities.as_slice())
            .http_err("Failed to update project entities", StatusCode::BAD_REQUEST)?;
    }

    Ok(empty_response())
}

async fn projects_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectsResponse>>> {
    let projects = tokio::task::spawn_blocking(move || {
        app.projects
            .all()?
            .into_iter()
            .filter(|project| can_enumerate_project(project, user.as_ref()))
            .map(|project| ProjectResponse::new(&app, project))
            .collect::<anyhow::Result<Vec<_>>>()
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_err("Failed to get projects", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(ProjectsResponse { projects })).into())
}

async fn project_handler(
    app: State<RouterState>,
    MaybeAuth(user): MaybeAuth,
    Path(project_id): Path<String>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectResponse>>> {
    let resp = tokio::task::spawn_blocking(move || {
        let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if !can_view_project(&project, user.as_ref()) {
            http_bail!(StatusCode::NOT_FOUND, "Project not found")
        }
        ProjectResponse::new(&app, project).http_err("Failed to get project", StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(resp)).into())
}

async fn settings_handler(
    app: State<RouterState>,
    Admin(_): Admin,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<CollectionSettings>>> {
    Ok(([(http::header::CACHE_CONTROL, "private")], Json(app.settings.global())).into())
}

async fn settings_update_handler(
    app: State<RouterState>,
    Admin(_): Admin,
    Json(settings): Json<CollectionSettings>,
) -> ApiResult<impl IntoApiResponse> {
    app.settings.update_global(&settings).http_err("Failed to update collection settings", StatusCode::BAD_REQUEST)?;

    Ok(empty_response())
}

async fn prune_handler(
    app: State<RouterState>,
    Admin(_): Admin,
    Json(req): Json<PruneRequest>,
) -> ApiResult<Json<PruneResponse>> {
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
    Admin(_): Admin,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ProjectDisplaySettings>>> {
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
    Admin(_): Admin,
    Json(mut settings): Json<ProjectDisplaySettings>,
) -> ApiResult<impl IntoApiResponse> {
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
    Admin(_): Admin,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<EntityCollectionSettingsResponse>>> {
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
    Admin(_): Admin,
    Json(mut settings): Json<EntityCollectionSettings>,
) -> ApiResult<impl IntoApiResponse> {
    if !app.entities.exists(&entity_id).http_err("Failed to get entity", StatusCode::INTERNAL_SERVER_ERROR)? {
        http_bail!(StatusCode::NOT_FOUND, "Entity not found")
    }

    settings.entity_id = entity_id;
    app.settings
        .update_entity(&settings)
        .http_err("Failed to update entity collection settings", StatusCode::BAD_REQUEST)?;

    Ok(empty_response())
}

async fn api_keys_handler(
    app: State<RouterState>,
    Admin(_): Admin,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ApiKeysResponse>>> {
    let keys = app
        .api_keys
        .all()
        .http_err("Failed to list API keys", StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(([(http::header::CACHE_CONTROL, "private, no-store")], Json(ApiKeysResponse { keys })).into())
}

async fn api_key_create_handler(
    app: State<RouterState>,
    Admin(user): Admin,
    Json(request): Json<CreateApiKeyRequest>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<CreateApiKeyResponse>>> {
    let (key, plaintext) = app
        .api_keys
        .create(&request.display_name, &request.entities, &request.projects, &request.permissions, request.expiration)
        .http_err("Failed to create API key", StatusCode::BAD_REQUEST)?;
    tracing::info!(key_id = key.id, actor = user.username, "Created API key");
    Ok((
        StatusCode::CREATED,
        [(http::header::CACHE_CONTROL, "private, no-store")],
        Json(CreateApiKeyResponse { key: key.into(), plaintext }),
    )
        .into())
}

async fn api_key_update_handler(
    app: State<RouterState>,
    Path(key_id): Path<String>,
    Admin(user): Admin,
    Json(request): Json<UpdateApiKeyRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if !app
        .api_keys
        .update(&key_id, &request.display_name, &request.entities, &request.projects, &request.permissions)
        .http_err("Failed to update API key", StatusCode::BAD_REQUEST)?
    {
        http_bail!(StatusCode::NOT_FOUND, "API key not found")
    }
    tracing::info!(key_id, actor = user.username, "Updated API key access");
    Ok(empty_response())
}

async fn api_key_delete_handler(
    app: State<RouterState>,
    Path(key_id): Path<String>,
    Admin(user): Admin,
) -> ApiResult<impl IntoApiResponse> {
    if !app.api_keys.delete(&key_id).http_err("Failed to delete API key", StatusCode::INTERNAL_SERVER_ERROR)? {
        http_bail!(StatusCode::NOT_FOUND, "API key not found")
    }
    tracing::info!(key_id, actor = user.username, "Deleted API key");
    Ok(empty_response())
}

async fn api_key_regenerate_handler(
    app: State<RouterState>,
    Path(key_id): Path<String>,
    Admin(user): Admin,
    Json(request): Json<RegenerateApiKeyRequest>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<CreateApiKeyResponse>>> {
    let Some((key, plaintext)) = app
        .api_keys
        .regenerate(&key_id, request.expiration)
        .http_err("Failed to regenerate API key", StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        http_bail!(StatusCode::NOT_FOUND, "API key not found")
    };
    tracing::info!(key_id, actor = user.username, "Regenerated API key");
    Ok((
        [(http::header::CACHE_CONTROL, "private, no-store")],
        Json(CreateApiKeyResponse { key: key.into(), plaintext }),
    )
        .into())
}

async fn project_delete_handler(
    app: State<RouterState>,
    Path(project_id): Path<String>,
    Admin(_): Admin,
) -> ApiResult<impl IntoApiResponse> {
    let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;

    app.projects.delete(&project.id).http_err("Failed to delete project", StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(empty_response())
}

async fn entities_handler(
    app: State<RouterState>,
    Admin(_): Admin,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<EntitiesResponse>>> {
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
                    visibility: project.visibility,
                })
                .collect(),
        });
    }

    Ok(([(http::header::CACHE_CONTROL, "private")], Json(EntitiesResponse { entities: resp })).into())
}

async fn entity_create_handler(
    app: State<RouterState>,
    Admin(_): Admin,
    Json(entity): Json<CreateEntityRequest>,
) -> ApiResult<Json<EntityResponse>> {
    app.entities
        .create(
            &Entity { id: entity.id.clone(), display_name: entity.display_name.clone() },
            entity.projects.as_slice(),
        )
        .http_err("Failed to create entity", StatusCode::BAD_REQUEST)?;

    Ok(Json(EntityResponse { id: entity.id, display_name: entity.display_name, projects: Vec::new() }))
}

async fn entity_update_handler(
    app: State<RouterState>,
    Path(entity_id): Path<String>,
    Admin(_): Admin,
    Json(entity): Json<UpdateEntityRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if let Some(display_name) = entity.display_name {
        app.entities
            .update(&Entity { id: entity_id.clone(), display_name })
            .http_err("Failed to update entity", StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(projects) = entity.projects {
        app.entities
            .update_projects(&entity_id, projects.as_slice())
            .http_err("Failed to update entity projects", StatusCode::BAD_REQUEST)?;
    }

    Ok(empty_response())
}

async fn entity_delete_handler(
    app: State<RouterState>,
    Path(entity_id): Path<String>,
    Admin(_): Admin,
) -> ApiResult<impl IntoApiResponse> {
    app.entities.delete(&entity_id).http_err("Failed to delete entity", StatusCode::INTERNAL_SERVER_ERROR)?;
    app.settings.reload().http_err("Failed to reload collection settings", StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(empty_response())
}
