use super::webext::*;
use crate::api::session_cookie;
use crate::app::App;
use crate::config::MAX_SESSION_AGE;

use lazy_static::lazy_static;
use poem::http::StatusCode;
use poem::web::cookie::{Cookie, CookieJar, SameSite};
use poem::web::{Data, Json};
use poem::{handler, post, session::Session, IntoResponse, Route};
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

pub fn router() -> Route {
    Route::new().at("/login", post(login_handler)).at("/logout", post(logout_handler))
}

#[derive(Deserialize)]
struct LoginParams {
    username: String,
    password: String,
}

#[handler]
async fn login_handler(
    Data(app): Data<&App>,
    Json(params): Json<LoginParams>,
    session: &Session,
    cookies: &CookieJar,
) -> poem::Result<impl IntoResponse> {
    if !app.check_login(&params.username, &params.password) {
        http_bail!(StatusCode::UNAUTHORIZED, "invalid username or password");
    }

    let mut public_cookie = PUBLIC_COOKIE.clone();
    public_cookie.set_value_str(params.username.clone());

    cookies.add(public_cookie);
    session.set("username", params.username);
    http_res!()
}

#[handler]
async fn logout_handler(session: &Session, cookies: &CookieJar) -> poem::Result<impl IntoResponse> {
    let mut cookie = PUBLIC_COOKIE.clone();
    cookie.make_removal();
    cookies.add(cookie);
    session_cookie().remove_cookie(cookies);

    session.purge();
    http_res!()
}
