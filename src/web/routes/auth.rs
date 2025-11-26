use std::sync::Arc;

use crate::app::Liwan;
use crate::app::models::UserRole;
use crate::utils::hash::session_token;
use crate::web::PoemErrExt;
use crate::web::session::{MAX_SESSION_AGE, PUBLIC_COOKIE, SESSION_COOKIE, SessionId, SessionUser};
use crate::web::webext::{ApiResult, EmptyResponse, http_bail};

use anyhow::anyhow;
use chrono::Utc;
use poem::http::StatusCode;
use poem::middleware::TowerLayerCompatExt;
use poem::web::{Data, cookie::CookieJar};
use poem::{Endpoint, EndpointExt};
use poem_openapi::payload::{Json, Response};
use poem_openapi::{Object, OpenApi};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tower::limit::RateLimitLayer;

#[derive(Serialize, Object)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Object)]
pub struct SetupRequest {
    pub token: String,
    pub username: String,
    pub password: String,
}

#[derive(Object, Serialize)]
pub struct MeResponse {
    pub username: String,
    pub role: UserRole,
}

fn login_rate_limit(ep: impl Endpoint + 'static) -> impl Endpoint {
    ep.with(RateLimitLayer::new(10, std::time::Duration::from_secs(10)).compat())
}

pub struct AuthApi;
#[OpenApi]
impl AuthApi {
    #[oai(path = "/auth/me", method = "get")]
    async fn me(&self, SessionUser(user): SessionUser) -> Response<Json<MeResponse>> {
        Response::new(Json(MeResponse { username: user.username, role: user.role })).header("Cache-Control", "private")
    }

    #[oai(path = "/auth/setup", method = "post", transform = "login_rate_limit")]
    async fn setup(&self, Data(app): Data<&Arc<Liwan>>, Json(params): Json<SetupRequest>) -> ApiResult<EmptyResponse> {
        let token = app.onboarding.token().http_status(StatusCode::INTERNAL_SERVER_ERROR)?.clone();

        if token != Some(params.token) {
            http_bail!(StatusCode::UNAUTHORIZED, "invalid setup token");
        }

        app.users
            .create(&params.username, &params.password, UserRole::Admin, &[])
            .http_err("failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

        app.onboarding
            .clear()
            .map_err(|_| anyhow!("onboarding lock poisoned"))
            .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/auth/login", method = "post", transform = "login_rate_limit")]
    async fn login(
        &self,
        Data(app): Data<&Arc<Liwan>>,
        Json(params): Json<LoginRequest>,
        cookies: &CookieJar,
    ) -> ApiResult<EmptyResponse> {
        let username = params.username.clone();

        let app2 = app.clone();
        let authorized =
            spawn_blocking(move || app2.users.check_login(&params.username, &params.password).unwrap_or(false))
                .await
                .unwrap_or(false);

        if !(authorized) {
            http_bail!(StatusCode::UNAUTHORIZED, "invalid username or password");
        }

        let session_id = session_token();
        let expires = Utc::now() + MAX_SESSION_AGE;
        app.sessions.create(&session_id, &username, expires).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut public_cookie = PUBLIC_COOKIE.clone();
        let mut session_cookie = SESSION_COOKIE.clone();
        public_cookie.set_secure(app.config.secure());
        public_cookie.set_value_str(username.clone());
        session_cookie.set_secure(app.config.secure());
        session_cookie.set_value_str(session_id);
        cookies.add(public_cookie);
        cookies.add(session_cookie);
        EmptyResponse::ok()
    }

    #[oai(path = "/auth/logout", method = "post")]
    async fn logout(
        &self,
        Data(app): Data<&Arc<Liwan>>,
        cookies: &CookieJar,
        session_id: Option<SessionId>,
    ) -> ApiResult<EmptyResponse> {
        if let Some(session_id) = session_id {
            let _ = app.sessions.delete(&session_id.0);
        }
        let mut public_cookie = PUBLIC_COOKIE.clone();
        let mut session_cookie = SESSION_COOKIE.clone();
        public_cookie.set_secure(app.config.secure());
        public_cookie.make_removal();
        session_cookie.set_secure(app.config.secure());
        session_cookie.make_removal();
        cookies.add(public_cookie);
        cookies.add(session_cookie);
        EmptyResponse::ok()
    }
}
