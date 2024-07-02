use super::{session_cookie, webext::*};
use crate::app::App;
use crate::config::MAX_SESSION_AGE;

use lazy_static::lazy_static;
use poem::http::StatusCode;
use poem::session::Session;
use poem::web::cookie::{Cookie, CookieJar, SameSite};
use poem::web::Data;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use serde::Deserialize;

lazy_static! {
    pub static ref PUBLIC_COOKIE: Cookie = {
        let mut public_cookie = Cookie::named("liwan-username");
        public_cookie.set_max_age(MAX_SESSION_AGE);
        public_cookie.set_http_only(false);
        public_cookie.set_path("/");
        public_cookie.set_same_site(SameSite::Strict);
        public_cookie
    };
}

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
        session: &Session,
        cookies: &CookieJar,
    ) -> APIResult<EmptyResponse> {
        if !app.check_login(&params.username, &params.password) {
            http_bail!(StatusCode::UNAUTHORIZED, "invalid username or password");
        }

        let mut public_cookie = PUBLIC_COOKIE.clone();
        public_cookie.set_value_str(params.username.clone());

        cookies.add(public_cookie);
        session.set("username", params.username);
        EmptyResponse::ok()
    }

    #[oai(path = "/auth/logout", method = "post")]
    async fn logout(&self, session: &Session, cookies: &CookieJar) -> APIResult<EmptyResponse> {
        let mut cookie = PUBLIC_COOKIE.clone();
        cookie.make_removal();
        cookies.add(cookie);
        session_cookie().remove_cookie(cookies);
        session.purge();
        EmptyResponse::ok()
    }
}
