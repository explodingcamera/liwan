use super::session::{SessionId, MAX_SESSION_AGE, PUBLIC_COOKIE, SESSION_COOKIE};
use super::webext::*;
use crate::app::App;
use crate::utils::hash::session_token;

use poem::http::StatusCode;
use poem::web::cookie::CookieJar;
use poem::web::Data;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use serde::Deserialize;

#[derive(Deserialize, Object)]
struct LoginRequest {
    username: String,
    password: String,
}

pub struct AuthApi;
#[OpenApi]
impl AuthApi {
    #[oai(path = "/auth/login", method = "post")]
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
