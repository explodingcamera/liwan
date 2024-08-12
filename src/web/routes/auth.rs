use crate::app::models::UserRole;
use crate::app::Liwan;
use crate::utils::hash::session_token;
use crate::web::session::{SessionId, SessionUser, MAX_SESSION_AGE, PUBLIC_COOKIE, SESSION_COOKIE};
use crate::web::webext::{http_bail, ApiResult, EmptyResponse};
use crate::web::PoemErrExt;

use eyre::eyre;
use poem::http::StatusCode;
use poem::middleware::TowerLayerCompatExt;
use poem::web::{cookie::CookieJar, Data};
use poem::{Endpoint, EndpointExt};
use poem_openapi::payload::{Json, Response};
use poem_openapi::{Object, OpenApi};
use serde::Deserialize;
use tower::limit::RateLimitLayer;

#[derive(Deserialize, Object)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize, Object)]
struct SetupRequest {
    token: String,
    username: String,
    password: String,
}

#[derive(Object)]
struct MeResponse {
    username: String,
    role: UserRole,
}

fn login_rate_limit(ep: impl Endpoint + 'static) -> impl Endpoint {
    ep.with(RateLimitLayer::new(10, std::time::Duration::from_secs(10)).compat())
}

pub(crate) struct AuthApi;
#[OpenApi]
impl AuthApi {
    #[oai(path = "/auth/me", method = "get")]
    async fn me(&self, SessionUser(user): SessionUser) -> Response<Json<MeResponse>> {
        Response::new(Json(MeResponse { username: user.username, role: user.role })).header("Cache-Control", "private")
    }

    #[oai(path = "/auth/setup", method = "post", transform = "login_rate_limit")]
    async fn setup(&self, Data(app): Data<&Liwan>, Json(params): Json<SetupRequest>) -> ApiResult<EmptyResponse> {
        let token = app.onboarding.token().http_status(StatusCode::INTERNAL_SERVER_ERROR)?.clone();

        if token != Some(params.token) {
            http_bail!(StatusCode::UNAUTHORIZED, "invalid setup token");
        }

        app.users
            .create(&params.username, &params.password, UserRole::Admin, &[])
            .http_err("failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

        app.onboarding
            .clear()
            .map_err(|_| eyre!("onboarding lock poisoned"))
            .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        EmptyResponse::ok()
    }

    #[oai(path = "/auth/login", method = "post", transform = "login_rate_limit")]
    async fn login(
        &self,
        Data(app): Data<&Liwan>,
        Json(params): Json<LoginRequest>,
        cookies: &CookieJar,
    ) -> ApiResult<EmptyResponse> {
        if !(app.users.check_login(&params.username, &params.password).unwrap_or(false)) {
            http_bail!(StatusCode::UNAUTHORIZED, "invalid username or password");
        }

        let session_id = session_token();
        let expires = chrono::Utc::now() + MAX_SESSION_AGE;
        app.sessions.create(&session_id, &params.username, expires).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut public_cookie = PUBLIC_COOKIE.clone();
        let mut session_cookie = SESSION_COOKIE.clone();
        public_cookie.set_secure(app.config.secure());
        public_cookie.set_value_str(params.username.clone());
        session_cookie.set_secure(app.config.secure());
        session_cookie.set_value_str(session_id);
        cookies.add(public_cookie);
        cookies.add(session_cookie);
        EmptyResponse::ok()
    }

    #[oai(path = "/auth/logout", method = "post")]
    async fn logout(
        &self,
        Data(app): Data<&Liwan>,
        cookies: &CookieJar,
        SessionId(session_id): SessionId,
    ) -> ApiResult<EmptyResponse> {
        app.sessions.delete(&session_id).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
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
