use std::time::Duration;

use aide::OperationInput;
use axum::{
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Utc;

use crate::web::RouterState;
use crate::{
    app::{
        Liwan,
        models::{User, UserRole},
    },
    utils::hash::session_token,
};

pub const MAX_SESSION_AGE: Duration = Duration::from_secs(24 * 60 * 60 * 14);

pub static PUBLIC_COOKIE_NAME: &str = "liwan-username";
pub static SESSION_COOKIE_NAME: &str = "liwan-session";

fn public_cookie(app: &Liwan) -> Cookie<'static> {
    let mut public_cookie = Cookie::new(PUBLIC_COOKIE_NAME, "");
    public_cookie.set_max_age(Some(MAX_SESSION_AGE.try_into().unwrap()));
    public_cookie.set_http_only(false);
    let path = app.config.base_path();
    public_cookie.set_path(if path.is_empty() { "/".to_string() } else { path.to_string() });
    public_cookie.set_same_site(SameSite::Strict);
    public_cookie.set_secure(app.config.secure());
    public_cookie
}

fn session_cookie(app: &Liwan) -> Cookie<'static> {
    let mut session_cookie = Cookie::new(SESSION_COOKIE_NAME, "");
    session_cookie.set_max_age(Some(MAX_SESSION_AGE.try_into().unwrap()));
    session_cookie.set_http_only(true);
    session_cookie.set_path(app.config.path("/api/dashboard"));
    session_cookie.set_same_site(SameSite::Strict);
    session_cookie.set_secure(app.config.secure());
    session_cookie
}

pub(crate) fn clear_session(app: &Liwan) -> CookieJar {
    let mut session_cookie = session_cookie(app);
    session_cookie.make_removal();
    let mut public_cookie = public_cookie(app);
    public_cookie.make_removal();
    CookieJar::new().add(session_cookie).add(public_cookie)
}

/// Creates a Liwan session and adds its browser cookies.
pub(crate) async fn issue_session(app: &Liwan, cookies: CookieJar, username: &str) -> anyhow::Result<CookieJar> {
    let session_id = session_token();
    let sessions = app.sessions.clone();
    let id = session_id.clone();
    let name = username.to_string();
    tokio::task::spawn_blocking(move || sessions.create(&id, &name, Utc::now() + MAX_SESSION_AGE)).await??;

    let mut public_cookie = public_cookie(app);
    let mut session_cookie = session_cookie(app);
    public_cookie.set_value(username.to_string());
    session_cookie.set_value(session_id);

    Ok(cookies.add(public_cookie).add(session_cookie))
}

#[derive(Debug, Clone)]
pub struct MaybeSessionId(pub Option<String>);

#[derive(Debug, Clone)]
pub struct Auth(pub User);

/// An authenticated administrator.
pub struct Admin(pub User);

#[derive(Debug, Clone)]
pub struct MaybeAuth(pub Option<User>);

impl OperationInput for Auth {}
impl OperationInput for Admin {}
impl OperationInput for MaybeAuth {}
impl OperationInput for MaybeSessionId {}

fn logout_response(app: &Liwan) -> Response {
    (clear_session(app), StatusCode::UNAUTHORIZED).into_response()
}

impl axum::extract::FromRequestParts<RouterState> for MaybeSessionId {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let session_cookie = jar.get(SESSION_COOKIE_NAME);
        let username_cookie = jar.get(PUBLIC_COOKIE_NAME);

        // log out if the cookies are in an inconsistent state (one is present but not the other)
        if let Some(username_cookie) = username_cookie
            && session_cookie.is_none()
        {
            let username = username_cookie.value();
            tracing::info!(username, "user has username cookie but no session cookie, logging out");
            return Err(logout_response(&state.app));
        }

        if let Some(session_cookie) = session_cookie
            && username_cookie.is_none()
        {
            let session_id = session_cookie.value().to_string();
            let sessions = state.sessions.clone();
            tracing::info!("user has session cookie but no username cookie, logging out");
            let _ = tokio::task::spawn_blocking(move || sessions.delete(&session_id)).await;
            return Err(logout_response(&state.app));
        }

        Ok(MaybeSessionId(jar.get(SESSION_COOKIE_NAME).map(|c| c.value().to_string())))
    }
}

impl axum::extract::FromRequestParts<RouterState> for Auth {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        let MaybeAuth(user) = MaybeAuth::from_request_parts(parts, state).await?;
        let user = user.ok_or_else(|| logout_response(&state.app))?;
        Ok(Auth(user))
    }
}

impl axum::extract::FromRequestParts<RouterState> for Admin {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        let Auth(user) = Auth::from_request_parts(parts, state).await?;
        if user.role != UserRole::Admin {
            return Err(super::webext::ApiError::from(StatusCode::FORBIDDEN).into_response());
        }
        Ok(Self(user))
    }
}

impl axum::extract::FromRequestParts<RouterState> for MaybeAuth {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        let MaybeSessionId(Some(session_id)) = MaybeSessionId::from_request_parts(parts, state).await? else {
            return Ok(MaybeAuth(None));
        };
        let sessions = state.sessions.clone();
        let user = tokio::task::spawn_blocking(move || sessions.get(&session_id))
            .await
            .map_err(|_| logout_response(&state.app))?
            .map_err(|_| logout_response(&state.app))?
            .ok_or_else(|| logout_response(&state.app))?;
        Ok(MaybeAuth(Some(user)))
    }
}
