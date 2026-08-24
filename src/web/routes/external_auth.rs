use std::{sync::LazyLock, time::Duration};

use aide::{
    UseApi,
    axum::{ApiRouter, IntoApiResponse, routing::*},
};
use axum::{
    Json,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    app::{ExternalAuthProvider, ExternalAuthSettings, models::UserRole},
    web::{
        RouterState,
        session::{Auth, issue_session},
        webext::{ApiResult, AxumErrExt, http_bail},
    },
};

const STATE_COOKIE_NAME: &str = "liwan-external-auth-state";
const CALLBACK_ERROR_PATH: &str = "/login?externalAuthError=1";

static STATE_COOKIE: LazyLock<Cookie<'static>> = LazyLock::new(|| {
    let mut cookie = Cookie::new(STATE_COOKIE_NAME, "");
    cookie.set_http_only(true);
    cookie.set_max_age(Some(Duration::from_secs(10 * 60).try_into().unwrap()));
    cookie.set_path("/api/dashboard/auth/external/callback");
    cookie.set_same_site(SameSite::Lax);
    cookie
});

pub fn router() -> ApiRouter<RouterState> {
    let start_limiter =
        GovernorConfigBuilder::default().per_second(1).burst_size(5).finish().expect("valid governor config");
    let callback_limiter =
        GovernorConfigBuilder::default().per_second(2).burst_size(5).finish().expect("valid governor config");
    let start_governor = start_limiter.limiter().clone();
    let callback_governor = callback_limiter.limiter().clone();
    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_hours(1));
        loop {
            interval.tick().await;
            start_governor.retain_recent();
            callback_governor.retain_recent();
        }
    });

    ApiRouter::new()
        .api_route("/auth/external", get(metadata))
        .api_route("/admin/auth", get(get_settings))
        .api_route("/admin/auth", put(update_settings))
        .merge(ApiRouter::new().api_route("/auth/external/start", get(start)).layer(GovernorLayer::new(start_limiter)))
        .merge(
            ApiRouter::new()
                .api_route("/auth/external/callback", get(callback))
                .layer(GovernorLayer::new(callback_limiter)),
        )
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ExternalAuthMetadata {
    enabled: bool,
    provider: ExternalAuthProvider,
    display_name: String,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ExternalAuthSettingsResponse {
    enabled: bool,
    provider: ExternalAuthProvider,
    display_name: String,
    client_id: String,
    client_secret_configured: bool,
    issuer_url: Option<String>,
    allowed_domain: Option<String>,
    allowed_organization: Option<String>,
    allow_user_creation: bool,
    callback_url: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct UpdateExternalAuthSettings {
    enabled: bool,
    provider: ExternalAuthProvider,
    display_name: String,
    client_id: String,
    #[serde(default)]
    client_secret: Option<String>,
    #[serde(default)]
    clear_client_secret: bool,
    issuer_url: Option<String>,
    allowed_domain: Option<String>,
    allowed_organization: Option<String>,
    allow_user_creation: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct StartQuery {
    #[serde(default = "default_return_to")]
    return_to: String,
}

#[derive(Deserialize, JsonSchema)]
struct CallbackQuery {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

fn default_return_to() -> String {
    "/".to_string()
}

async fn metadata(app: State<RouterState>) -> ApiResult<UseApi<impl IntoApiResponse, Json<ExternalAuthMetadata>>> {
    let settings = app.external_auth.settings().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let onboarded = app.onboarding.token().http_status(StatusCode::INTERNAL_SERVER_ERROR)?.is_none();
    Ok(Json(ExternalAuthMetadata {
        enabled: settings.enabled && onboarded,
        provider: settings.provider,
        display_name: settings.display_name,
    })
    .into())
}

async fn get_settings(
    app: State<RouterState>,
    Auth(user): Auth,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ExternalAuthSettingsResponse>>> {
    require_admin(user.role)?;
    let settings = app.external_auth.settings().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(settings_response(&app, settings)).into())
}

async fn update_settings(
    app: State<RouterState>,
    Auth(user): Auth,
    Json(request): Json<UpdateExternalAuthSettings>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<ExternalAuthSettingsResponse>>> {
    require_admin(user.role)?;
    if request.clear_client_secret && request.client_secret.is_some() {
        http_bail!(StatusCode::BAD_REQUEST, "clientSecret and clearClientSecret cannot both be set");
    }
    if request.client_secret.as_deref().is_some_and(|secret| secret.trim().is_empty()) {
        http_bail!(StatusCode::BAD_REQUEST, "clientSecret cannot be empty, use clearClientSecret to remove it");
    }

    let existing = app.external_auth.settings().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    let client_secret =
        if request.clear_client_secret { None } else { request.client_secret.or(existing.client_secret) };
    let settings = ExternalAuthSettings {
        enabled: request.enabled,
        provider: request.provider,
        display_name: request.display_name,
        client_id: request.client_id,
        client_secret,
        issuer_url: request.issuer_url,
        allowed_domain: request.allowed_domain,
        allowed_organization: request.allowed_organization,
        allow_user_creation: request.allow_user_creation,
    };
    if let Err(error) = app.external_auth.update_settings(&settings).await {
        tracing::debug!(%error, "external authentication settings validation failed");
        http_bail!(StatusCode::BAD_REQUEST, "{error}");
    }
    let settings = app.external_auth.settings().http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(settings_response(&app, settings)).into())
}

async fn start(
    app: State<RouterState>,
    cookies: CookieJar,
    Query(query): Query<StartQuery>,
) -> ApiResult<UseApi<Response, ()>> {
    if app.onboarding.token().http_status(StatusCode::INTERNAL_SERVER_ERROR)?.is_some() {
        http_bail!(StatusCode::NOT_FOUND, "external authentication is unavailable");
    }
    let start = app
        .external_auth
        .begin(query.return_to)
        .await
        .http_err("external authentication is unavailable", StatusCode::BAD_REQUEST)?;
    let mut state_cookie = STATE_COOKIE.clone();
    state_cookie.set_secure(app.config.secure());
    state_cookie.set_value(start.state);
    Ok((cookies.add(state_cookie), Redirect::to(start.authorization_url.as_str())).into_response().into())
}

async fn callback(
    app: State<RouterState>,
    cookies: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> UseApi<Response, ()> {
    let cookie_state = cookies.get(STATE_COOKIE_NAME).map(|cookie| cookie.value().to_string());
    let mut removal = STATE_COOKIE.clone();
    removal.set_secure(app.config.secure());
    removal.make_removal();
    let cookies = cookies.add(removal);

    let Some(cookie_state) = cookie_state else {
        tracing::debug!("external authentication callback has no state cookie");
        return (cookies, Redirect::to(CALLBACK_ERROR_PATH)).into_response().into();
    };
    if query.error.is_some() || query.state.as_deref() != Some(cookie_state.as_str()) {
        app.external_auth.cancel(&cookie_state);
        tracing::debug!(provider_error = ?query.error, "external authentication callback was rejected");
        return (cookies, Redirect::to(CALLBACK_ERROR_PATH)).into_response().into();
    }
    let Some(code) = query.code else {
        app.external_auth.cancel(&cookie_state);
        tracing::debug!("external authentication callback has no authorization code");
        return (cookies, Redirect::to(CALLBACK_ERROR_PATH)).into_response().into();
    };

    let response = match app.external_auth.finish(&cookie_state, code).await {
        Ok(login) => match issue_session(&app, cookies.clone(), &login.username) {
            Ok(cookies) => (cookies, Redirect::to(&login.return_to)).into_response(),
            Err(error) => {
                tracing::error!(%error, "failed to create external authentication session");
                (cookies, Redirect::to(CALLBACK_ERROR_PATH)).into_response()
            }
        },
        Err(error) => {
            tracing::debug!(%error, "external authentication callback failed");
            (cookies, Redirect::to(CALLBACK_ERROR_PATH)).into_response()
        }
    };
    response.into()
}

fn require_admin(role: UserRole) -> ApiResult<()> {
    if role != UserRole::Admin {
        http_bail!(StatusCode::FORBIDDEN, "Forbidden");
    }
    Ok(())
}

fn settings_response(app: &RouterState, settings: ExternalAuthSettings) -> ExternalAuthSettingsResponse {
    ExternalAuthSettingsResponse {
        enabled: settings.enabled,
        provider: settings.provider,
        display_name: settings.display_name,
        client_id: settings.client_id,
        client_secret_configured: settings.client_secret.is_some(),
        issuer_url: settings.issuer_url,
        allowed_domain: settings.allowed_domain,
        allowed_organization: settings.allowed_organization,
        allow_user_creation: settings.allow_user_creation,
        callback_url: app.external_auth.callback_url().to_string(),
    }
}
