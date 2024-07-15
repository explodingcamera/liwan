use super::{session::SessionUser, webext::*};
use crate::app::{
    models::{Entity, Project, UserRole},
    App,
};
use poem::{http::StatusCode, web::Data};
use poem_openapi::{
    param::Path,
    payload::{Json, Response},
    Object, OpenApi,
};

pub(crate) struct AdminAPI;

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
struct ProjectRequest {
    display_name: String,
    public: bool,
    secret: Option<String>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
pub(crate) struct ProjectResponse {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) entities: Vec<ProjectEntity>,
    pub(crate) public: bool,
}

#[derive(Object)]
pub(crate) struct ProjectsResponse {
    pub(crate) projects: Vec<ProjectResponse>,
}

#[derive(Object)]
#[oai(rename_all = "camelCase")]
pub(crate) struct ProjectEntity {
    pub(crate) id: String,
    pub(crate) display_name: String,
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
struct EntitiesResponse {
    entities: Vec<EntityResponse>,
}

#[OpenApi]
impl AdminAPI {
    #[oai(path = "/users", method = "get")]
    async fn users_handler(
        &self,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Response<Json<UsersResponse>>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        let users = app
            .users()
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
        Data(app): Data<&App>,
        SessionUser(session_user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if session_user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        if username == session_user.username && user.role != session_user.role {
            http_bail!(StatusCode::FORBIDDEN, "Cannot change own role")
        }

        app.user_update(&username, user.role, user.projects.as_slice())
            .http_err("Failed to update user", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
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

    #[oai(path = "/project/:project_id", method = "post")]
    async fn project_create_handler(
        &self,
        Json(project): Json<ProjectRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.project_create(&Project {
            id: project_id,
            display_name: project.display_name,
            public: project.public,
            secret: project.secret,
        })
        .http_err("Failed to create project", StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/project/:project_id", method = "put")]
    async fn project_update_handler(
        &self,
        Json(project): Json<ProjectRequest>,
        Path(project_id): Path<String>,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<EmptyResponse> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.project_update(&Project {
            id: project_id,
            display_name: project.display_name,
            public: project.public,
            secret: project.secret,
        })
        .http_err("Failed to update project", StatusCode::INTERNAL_SERVER_ERROR)?;
        EmptyResponse::ok()
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

    #[oai(path = "/entities", method = "get")]
    async fn entities_handler(
        &self,
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Response<Json<EntitiesResponse>>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        let entities = app.entities().http_err("Failed to get entities", StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut resp = Vec::new();
        for entity in entities {
            resp.push(EntityResponse {
                id: entity.id.clone(),
                display_name: entity.display_name.clone(),
                projects: app
                    .entity_projects(&entity.id)
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
        Data(app): Data<&App>,
        SessionUser(user): SessionUser,
    ) -> ApiResult<Json<EntityResponse>> {
        if user.role != UserRole::Admin {
            http_bail!(StatusCode::FORBIDDEN, "Forbidden")
        }

        app.entity_create(
            &Entity { id: entity.id.clone(), display_name: entity.display_name.clone() },
            entity.projects.as_slice(),
        )
        .http_err("Failed to create entity", StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(EntityResponse { id: entity.id, display_name: entity.display_name, projects: Vec::new() }))
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
}
