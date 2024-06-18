use super::webext::*;
use crate::app::App;
use crate::config::MAX_SESSION_AGE;

use poem::http::StatusCode;
use poem::web::cookie::{Cookie, CookieJar};
use poem::web::{Data, Json};
use poem::{handler, post, session::Session, IntoResponse, Route};
use serde::Deserialize;

pub fn router() -> Route {
    Route::new().at("/login", post(login_handler)).at("/logout", post(logout_handler))
}

#[derive(Deserialize)]
struct LoginParams {
    username: String,
    password: String,
}

#[handler]
pub(super) async fn login_handler(
    Data(app): Data<&App>,
    Json(params): Json<LoginParams>,
    session: &Session,
    cookies: &CookieJar,
) -> poem::Result<impl IntoResponse> {
    session.purge();
    if !app.check_login(&params.username, &params.password) {
        http_bail!("invalid username or password", StatusCode::UNAUTHORIZED);
    }

    let mut public_cookie = Cookie::new_with_str("username", params.username.clone());
    public_cookie.set_max_age(MAX_SESSION_AGE);
    public_cookie.set_http_only(false);
    cookies.add(public_cookie);
    session.set("username", params.username);
    http_res!()
}

#[handler]
pub(super) async fn logout_handler(
    session: &Session,
    cookies: &CookieJar,
) -> poem::Result<impl IntoResponse> {
    session.purge();
    cookies.remove("username");
    http_res!()
}
