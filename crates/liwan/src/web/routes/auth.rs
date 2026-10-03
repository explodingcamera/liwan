use aide::axum::{ApiRouter, IntoApiResponse, routing::*};
use axum::{Json, extract::State};
use axum_extra::extract::CookieJar;
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    PASSWORD_MIN_LENGTH,
    app::models::UserRole,
    config::Config,
    web::{
        MaybeSessionId, RouterState,
        session::{Auth, clear_session, issue_session},
        webext::{ApiResult, AxumErrExt, ClientIpKeyExtractor, empty_response, http_bail},
    },
};

pub fn router(config: &Config) -> ApiRouter<RouterState> {
    let limiter = GovernorConfigBuilder::default()
        .per_second(2)
        .burst_size(5)
        .key_extractor(ClientIpKeyExtractor::new(config))
        .finish()
        .expect("valid governor config");

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
        .api_route("/auth/me/password", put(update_password))
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

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ChangePasswordRequest {
    password: String,
    current_password: String,
}

#[derive(Serialize, JsonSchema)]
pub struct MeResponse {
    pub username: String,
    pub role: UserRole,
}

async fn me(Auth(user): Auth) -> Json<MeResponse> {
    Json(MeResponse { username: user.username, role: user.role })
}

async fn setup(app: State<RouterState>, Json(params): Json<SetupRequest>) -> ApiResult<impl IntoApiResponse> {
    if params.password.len() < PASSWORD_MIN_LENGTH {
        http_bail!(StatusCode::BAD_REQUEST, "password must be at least 8 characters long");
    }

    let completed = spawn_blocking(move || {
        app.onboarding
            .complete_setup(&params.token, || app.users.create(&params.username, &params.password, UserRole::Admin))
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_err("failed to complete setup", StatusCode::INTERNAL_SERVER_ERROR)?;
    if !completed {
        http_bail!(StatusCode::UNAUTHORIZED, "invalid setup token");
    }

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

    let cookies = issue_session(&app, cookies, &username).await.http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((cookies, empty_response()))
}

async fn update_password(
    app: State<RouterState>,
    Auth(user): Auth,
    MaybeSessionId(session_id): MaybeSessionId,
    Json(params): Json<ChangePasswordRequest>,
) -> ApiResult<impl IntoApiResponse> {
    if params.password.len() < PASSWORD_MIN_LENGTH {
        http_bail!(StatusCode::BAD_REQUEST, "password must be at least 8 characters long");
    }
    let users = app.users.clone();
    let changed = spawn_blocking(move || {
        users.update_password(&user.username, &params.password, Some(&params.current_password), session_id.as_deref())
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_err("Failed to update password", StatusCode::INTERNAL_SERVER_ERROR)?;
    if !changed {
        http_bail!(StatusCode::BAD_REQUEST, "Current password is incorrect");
    }
    Ok(empty_response())
}

async fn logout(
    app: State<RouterState>,
    MaybeSessionId(session_id): MaybeSessionId,
) -> ApiResult<impl IntoApiResponse> {
    if let Some(session_id) = session_id {
        let sessions = app.sessions.clone();
        match spawn_blocking(move || sessions.delete(&session_id)).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "failed to expire logout session"),
            Err(error) => tracing::warn!(%error, "failed to run logout session expiry"),
        }
    }
    Ok((clear_session(&app), empty_response()))
}
