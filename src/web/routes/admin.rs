use std::sync::Arc;

use crate::app::Liwan;
use crate::app::models::{Entity, Project, UserRole};
use crate::utils::validate::can_access_project;
use crate::web::{
    session::SessionUser,
    webext::{ApiResult, EmptyResponse, PoemErrExt, http_bail},
};

use poem::{http::StatusCode, web::Data};
use poem_openapi::param::Path;
use poem_openapi::payload::{Json, Response};
use poem_openapi::{Object, OpenApi};

pub struct AdminAPI;

#[derive(Debug, Object, Clone)]
struct CreateUserRequest {
    username: String,
    password: String,
    role: UserRole,
}

#[derive(Object)]
struct UpdateUserRequest {
    role: UserRole,
    projects: Vec<String>,
}

#[derive(Object)]
struct UserResponse {
    username: String,
    role: UserRole,
    projects: Vec<String>,
}

#[derive(Object)]
struct UsersResponse {
    users: Vec<UserResponse>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct UpdateProjectRequest {
    project: Option<UpdateProjectInfo>,
    entities: Option<Vec<String>>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct UpdateProjectInfo {
    display_name: String,
    public: bool,
    secret: Option<String>,
}

#[derive(Object)]
struct UpdatePasswordRequest {
    password: String,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct CreateProjectRequest {
    display_name: String,
    public: bool,
    secret: Option<String>,
    entities: Vec<String>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
pub struct ProjectResponse {
    pub id: String,
    pub display_name: String,
    pub entities: Vec<ProjectEntity>,
    pub public: bool,
}

#[derive(Object)]
pub struct ProjectsResponse {
    pub projects: Vec<ProjectResponse>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
pub struct ProjectEntity {
    pub id: String,
    pub display_name: String,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct EntityResponse {
    id: String,
    display_name: String,
    projects: Vec<EntityProject>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct EntityProject {
    id: String,
    display_name: String,
    public: bool,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct CreateEntityRequest {
    id: String,
    display_name: String,
    projects: Vec<String>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
struct UpdateEntityRequest {
    display_name: Option<String>,
    projects: Option<Vec<String>>,
}

#[derive(Object)]
struct EntitiesResponse {
    entities: Vec<EntityResponse>,
}

#[OpenApi]
impl AdminAPI {
    #[oai(path = "/users", method = "get")]
    async fn users_handler(
        &self,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Response<Json<UsersResponse>>> {
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

        Ok(Response::new(Json(UsersResponse { users })).header("Cache-Control", "private"))
    }

    #[oai(path = "/user/:username", method = "put")]
    async fn user_update_handler(
        &self,
        Path(username): Path<String>,
        Json(user): Json<UpdateUserRequest>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        if username == session_user.username && user.role != session_user.role {
            http_bail!(StatusCode::FORBIDDEN, "Cannot change own role")
        }

        app.users
            .update(&username, user.role, user.projects.as_slice())
            .http_err("Failed to update user", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/user/:username/password", method = "put")]
    async fn user_password_handler(
        &self,
        Path(username): Path<String>,
        Json(password): Json<UpdatePasswordRequest>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin || username != session_user.username {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.users
            .update_password(&username, &password.password)
            .http_err("Failed to update password", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/user/:username", method = "delete")]
    async fn user_delete_handler(
        &self,
        Path(username): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        if username == session_user.username {
            http_bail!(StatusCode::FORBIDDEN, "Cannot delete own user")
        }

        app.users.delete(&username).http_err("Failed to delete user", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/user", method = "post")]
    async fn user_create_handler(
        &self,
        Json(user): Json<CreateUserRequest>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        let app = app.clone();
        tokio::task::spawn_blocking(move || app.users.create(&user.username, &user.password, user.role, &[]))
            .await
            .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?
            .http_err("Failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/project/:project_id", method = "post")]
    async fn project_create_handler(
        &self,
        Json(project): Json<CreateProjectRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
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

        EmptyResponse::ok()
    }

    #[oai(path = "/project/:project_id", method = "put")]
    async fn project_update_handler(
        &self,
        Json(req): Json<UpdateProjectRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
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

        EmptyResponse::ok()
    }

    #[oai(path = "/projects", method = "get")]
    async fn projects_handler(
        &self,
        Data(app): Data<&Arc<Liwan>>,
        user: Option<SessionUser>,
    ) -> ApiResult<Response<Json<ProjectsResponse>>> {
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

        Ok(Response::new(Json(ProjectsResponse { projects: resp })).header("Cache-Control", "private"))
    }

    #[oai(path = "/project/:project_id", method = "get")]
    async fn project_handler(
        &self,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        user: Option<SessionUser>,
    ) -> ApiResult<Response<Json<ProjectResponse>>> {
        let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if !can_access_project(&project, user.as_ref()) {
            return Err(StatusCode::NOT_FOUND.into());
        }

        Ok(Response::new(Json(ProjectResponse {
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
        }))
        .header("Cache-Control", "private"))
    }

    #[oai(path = "/project/:project_id", method = "delete")]
    async fn project_delete_handler(
        &self,
        Path(project_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        let project = app.projects.get(&project_id).http_status(StatusCode::NOT_FOUND)?;
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.projects.delete(&project.id).http_err("Failed to delete project", StatusCode::INTERNAL_SERVER_ERROR)?;
        EmptyResponse::ok()
    }

    #[oai(path = "/entities", method = "get")]
    async fn entities_handler(
        &self,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Response<Json<EntitiesResponse>>> {
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

        Ok(Response::new(Json(EntitiesResponse { entities: resp })).header("Cache-Control", "private"))
    }

    #[oai(path = "/entity", method = "post")]
    async fn entity_create_handler(
        &self,
        Json(entity): Json<CreateEntityRequest>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
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

    #[oai(path = "/entity/:entity_id", method = "put")]
    async fn entity_update_handler(
        &self,
        Path(entity_id): Path<String>,
        Json(entity): Json<UpdateEntityRequest>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
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

        EmptyResponse::ok()
    }

    #[oai(path = "/entity/:entity_id", method = "delete")]
    async fn entity_delete_handler(
        &self,
        Path(entity_id): Path<String>,
        Data(app): Data<&Arc<Liwan>>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.entities.delete(&entity_id).http_err("Failed to delete entity", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }
}
