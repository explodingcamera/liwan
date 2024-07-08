use super::session::{SessionId, MAX_SESSION_AGE, PUBLIC_COOKIE, SESSION_COOKIE};
use super::webext::*;
use crate::app::models::UserRole;
use crate::app::App;
use crate::utils::hash::session_token;

use poem::http::StatusCode;
use poem::middleware::TowerLayerCompatExt;
use poem::web::{cookie::CookieJar, Data};
use poem::{Endpoint, EndpointExt};
use poem_openapi::payload::Json;
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

fn login_rate_limit(ep: impl Endpoint + 'static) -> impl Endpoint {
    ep.with(RateLimitLayer::new(10, std::time::Duration::from_secs(10)).compat())
}

pub struct AuthApi;
#[OpenApi]
impl AuthApi {
    #[oai(path = "/auth/setup", method = "post", transform = "login_rate_limit")]
    async fn setup(&self, Data(app): Data<&App>, Json(params): Json<SetupRequest>) -> APIResult<EmptyResponse> {
        let token = app.onboarding.read().http_internal("internal error")?.clone();
        if token != Some(params.token) {
            http_bail!(StatusCode::UNAUTHORIZED, "invalid setup token");
        }

        app.user_create(&params.username, &params.password, UserRole::Admin, vec![]).http_internal("internal error")?;
        *app.onboarding.write().http_internal("internal error")? = None;
        EmptyResponse::ok()
    }

    #[oai(path = "/auth/login", method = "post", transform = "login_rate_limit")]
    async fn login(
        &self,
        Data(app): Data<&App>,
        Json(params): Json<LoginRequest>,
        cookies: &CookieJar,
    ) -> APIResult<EmptyResponse> {
        if !(app.check_login(&params.username, &params.password).unwrap_or(false)) {
            http_bail!(StatusCode::UNAUTHORIZED, "invalid username or password");
        }

        let session_id = session_token();
        let expires = chrono::Utc::now() + MAX_SESSION_AGE;
        app.session_create(&session_id, &params.username, expires).http_internal("internal error")?;

        let mut public_cookie = PUBLIC_COOKIE.clone();
        let mut session_cookie = SESSION_COOKIE.clone();
        public_cookie.set_value_str(params.username.clone());
        session_cookie.set_value_str(session_id);
        cookies.add(public_cookie);
        cookies.add(session_cookie);
        EmptyResponse::ok()
    }

    #[oai(path = "/auth/logout", method = "post")]
    async fn logout(
        &self,
        Data(app): Data<&App>,
        cookies: &CookieJar,
        SessionId(session_id): SessionId,
    ) -> APIResult<EmptyResponse> {
        app.session_delete(&session_id).http_internal("internal error")?;
        let mut public_cookie = PUBLIC_COOKIE.clone();
        let mut session_cookie = SESSION_COOKIE.clone();
        public_cookie.make_removal();
        session_cookie.make_removal();
        cookies.add(public_cookie);
        cookies.add(session_cookie);
        EmptyResponse::ok()
    }
}
