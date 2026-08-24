use aide::{
    UseApi,
    axum::{ApiRouter, IntoApiResponse, routing::*},
};
use anyhow::Context;
use axum::{Json, extract::State};
use axum_extra::extract::CookieJar;
use http::{StatusCode, header};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    PASSWORD_MIN_LENGTH,
    app::models::UserRole,
    web::{
        MaybeSessionId, RouterState,
        session::{Auth, LOGOUT_COOKIES, issue_session},
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
        .api_route("/auth/me", get(me))
        .api_route("/auth/setup", post(setup))
        .api_route("/auth/logout", post(logout))
        .merge(ApiRouter::new().api_route("/auth/login", post(login)).layer(GovernorLayer::new(limiter)))
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

async fn me(Auth(user): Auth) -> UseApi<impl IntoApiResponse, Json<MeResponse>> {
    ([(header::CACHE_CONTROL, "private")], Json(MeResponse { username: user.username, role: user.role })).into()
}

async fn setup(app: State<RouterState>, Json(params): Json<SetupRequest>) -> ApiResult<impl IntoApiResponse> {
    let token = app.onboarding.token().http_status(StatusCode::INTERNAL_SERVER_ERROR)?.clone();

    if token != Some(params.token) {
        http_bail!(StatusCode::UNAUTHORIZED, "invalid setup token");
    }

    if params.password.len() < PASSWORD_MIN_LENGTH {
        http_bail!(StatusCode::BAD_REQUEST, "password must be at least 8 characters long");
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

    let cookies = issue_session(&app, cookies, &username).http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((cookies, empty_response()))
}

async fn logout(
    app: State<RouterState>,
    MaybeSessionId(session_id): MaybeSessionId,
) -> ApiResult<impl IntoApiResponse> {
    if let Some(session_id) = session_id {
        let _ = app.sessions.delete(&session_id);
    }
    Ok((LOGOUT_COOKIES.clone(), empty_response()))
}
