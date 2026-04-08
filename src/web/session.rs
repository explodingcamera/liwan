use std::{sync::LazyLock, time::Duration};

use aide::OperationInput;
use axum::{
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};

use crate::app::models::User;
use crate::web::RouterState;

pub const MAX_SESSION_AGE: Duration = Duration::from_secs(24 * 60 * 60 * 14);

pub static PUBLIC_COOKIE_NAME: &str = "liwan-username";
pub static SESSION_COOKIE_NAME: &str = "liwan-session";

pub static PUBLIC_COOKIE: LazyLock<Cookie<'static>> = LazyLock::new(|| {
    let mut public_cookie = Cookie::new(PUBLIC_COOKIE_NAME, "");
    public_cookie.set_max_age(Some(MAX_SESSION_AGE.try_into().unwrap()));
    public_cookie.set_http_only(false);
    public_cookie.set_path("/");
    public_cookie.set_same_site(SameSite::Strict);
    public_cookie
});

pub static SESSION_COOKIE: LazyLock<Cookie<'static>> = LazyLock::new(|| {
    let mut session_cookie = Cookie::new(SESSION_COOKIE_NAME, "");
    session_cookie.set_max_age(Some(MAX_SESSION_AGE.try_into().unwrap()));
    session_cookie.set_http_only(true);
    session_cookie.set_path("/api/dashboard");
    session_cookie.set_same_site(SameSite::Strict);
    session_cookie
});

pub static LOGOUT_COOKIES: LazyLock<CookieJar> = LazyLock::new(|| {
    let mut session_cookie = SESSION_COOKIE.clone();
    session_cookie.make_removal();
    let mut public_cookie = PUBLIC_COOKIE.clone();
    public_cookie.make_removal();
    CookieJar::new().add(session_cookie).add(public_cookie)
});

#[derive(Debug, Clone)]
pub struct MaybeSessionId(pub Option<String>);

#[derive(Debug, Clone)]
pub struct Auth(pub User);

#[derive(Debug, Clone)]
pub struct MaybeAuth(pub Option<User>);

impl OperationInput for Auth {}
impl OperationInput for MaybeAuth {}
impl OperationInput for MaybeSessionId {}

fn logout_response() -> Response {
    (LOGOUT_COOKIES.clone(), StatusCode::UNAUTHORIZED).into_response()
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
            return Err(logout_response());
        }

        if let Some(session_cookie) = session_cookie
            && username_cookie.is_none()
        {
            let session_id = session_cookie.value();
            tracing::info!(session_id, "user has session cookie but no username cookie, logging out");
            let _ = state.app.sessions.delete(session_cookie.value());
            return Err(logout_response());
        }

        Ok(MaybeSessionId(jar.get(SESSION_COOKIE_NAME).map(|c| c.value().to_string())))
    }
}

impl axum::extract::FromRequestParts<RouterState> for Auth {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        let session_id = MaybeSessionId::from_request_parts(parts, state).await?.0.ok_or_else(logout_response)?;
        let user = state.app.sessions.get(&session_id).map_err(|_| logout_response())?.ok_or_else(logout_response)?;
        Ok(Auth(user))
    }
}

impl axum::extract::FromRequestParts<RouterState> for MaybeAuth {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        let MaybeSessionId(Some(session_id)) = MaybeSessionId::from_request_parts(parts, state).await? else {
            return Ok(MaybeAuth(None));
        };
        let user = state.app.sessions.get(&session_id).map_err(|_| logout_response())?.ok_or_else(logout_response)?;
        Ok(MaybeAuth(Some(user)))
    }
}
