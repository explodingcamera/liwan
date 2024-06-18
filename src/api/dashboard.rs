use super::webutils::*;
use crate::app::App;
use crate::config::MAX_SESSION_AGE;
use crate::reports::{self, DateRange, Dimension, Metric};

use poem::web::cookie::{Cookie, CookieJar};
use poem::web::{Data, Json};
use poem::{handler, http::StatusCode, session::Session, IntoResponse};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[handler]
pub(super) async fn test_handler(Data(app): Data<&App>) -> impl IntoResponse {
    let res = reports::dimension_report(
        &app.conn().unwrap(),
        &["blog"],
        "pageview",
        DateRange { start: chrono::Utc::now() - chrono::Duration::days(7), end: chrono::Utc::now() },
        Dimension::Path,
        &[],
        Metric::UniqueVisitors,
    )
    .unwrap();
    Json(json!({ "status": "ok", "data": res }))
}

#[derive(serde::Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub display_name: String,
    pub entities: HashMap<String, String>,
}

#[handler]
pub(super) async fn groups_handler(
    Data(app): Data<&App>,
    session: &Session,
) -> poem::Result<impl IntoResponse> {
    let user = auth(session, app)?;
    let groups = app.resolve_user_groups(user).http_err("user not found", StatusCode::NOT_FOUND)?;
    http_res!(groups)
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
