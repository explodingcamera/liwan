use aide::axum::{ApiRouter, IntoApiResponse, routing::*};
use anyhow::Context;
use axum::{Json, extract::State};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use http::{StatusCode, header};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    app::models::UserRole,
    utils::hash::session_token,
    web::{
        MaybeExtract, RouterState, SessionUser,
        session::{MAX_SESSION_AGE, PUBLIC_COOKIE, SESSION_COOKIE, SessionId},
        webext::{ApiResult, AxumErrExt, empty_response, http_bail},
    },
};

pub fn router() -> ApiRouter<RouterState> {
    let limiter = GovernorConfigBuilder::default().per_second(2).burst_size(5).finish().expect("valid governor config");

    let governor_limiter = limiter.limiter().clone();
    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_hours(1));
        loop {
            interval.tick().await;
            governor_limiter.retain_recent();
        }
    });

    ApiRouter::new()
        .layer(GovernorLayer::new(limiter))
        .api_route("/auth/me", get(me))
        .api_route("/auth/setup", post(setup))
        .api_route("/auth/login", post(login))
        .api_route("/auth/logout", post(logout))
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct SetupRequest {
    pub token: String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, JsonSchema)]
pub struct MeResponse {
    pub username: String,
    pub role: UserRole,
}

async fn me(SessionUser(user): SessionUser) -> impl IntoApiResponse {
    ([(header::CACHE_CONTROL, "private")], Json(MeResponse { username: user.username, role: user.role }))
}

async fn setup(app: State<RouterState>, Json(params): Json<SetupRequest>) -> ApiResult<impl IntoApiResponse> {
    let token = app.onboarding.token().http_status(StatusCode::INTERNAL_SERVER_ERROR)?.clone();

    if token != Some(params.token) {
        http_bail!(StatusCode::UNAUTHORIZED, "invalid setup token");
    }

    app.users
        .create(&params.username, &params.password, UserRole::Admin, &[])
        .http_err("failed to create user", StatusCode::INTERNAL_SERVER_ERROR)?;

    app.onboarding.clear().context("onboarding lock poisoned").http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(empty_response())
}

async fn login(
    app: State<RouterState>,
    cookies: CookieJar,
    Json(params): Json<LoginRequest>,
) -> ApiResult<impl IntoApiResponse> {
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
    public_cookie.set_value(username.clone());
    session_cookie.set_secure(app.config.secure());
    session_cookie.set_value(session_id);

    let cookies = cookies.add(public_cookie).add(session_cookie);
    Ok((cookies, empty_response()))
}

async fn logout(
    app: State<RouterState>,
    cookies: CookieJar,
    MaybeExtract(session_id): MaybeExtract<SessionId>,
) -> ApiResult<impl IntoApiResponse> {
    if let Some(session_id) = session_id {
        let _ = app.sessions.delete(&session_id.0);
    }

    let mut public_cookie = PUBLIC_COOKIE.clone();
    let mut session_cookie = SESSION_COOKIE.clone();
    public_cookie.set_secure(app.config.secure());
    public_cookie.make_removal();
    session_cookie.set_secure(app.config.secure());
    session_cookie.make_removal();

    let cookies = cookies.add(public_cookie).add(session_cookie);
    Ok((cookies, empty_response()))
}
