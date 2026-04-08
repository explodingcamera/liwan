use std::{sync::LazyLock, time::Duration};

use aide::OperationInput;
use axum::{
    extract::FromRequestParts,
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

#[derive(Debug, Clone)]
pub struct SessionId(pub String);

#[derive(Debug, Clone)]
pub struct SessionUser(pub User);

// aide doesn't seem to support Option<T> extraction yet
#[derive(Debug, Clone)]
pub struct MaybeExtract<T>(pub Option<T>);

impl OperationInput for SessionId {}
impl OperationInput for SessionUser {}
impl<T> OperationInput for MaybeExtract<T> {}

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for SessionId {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        let session_cookie = jar.get(SESSION_COOKIE_NAME);
        let username_cookie = jar.get(PUBLIC_COOKIE_NAME);

        if username_cookie.is_some() && session_cookie.is_none() {
            let mut cookie = PUBLIC_COOKIE.clone();
            cookie.make_removal();
            return Err(Response::builder()
                .header("Set-Cookie", cookie.encoded().to_string())
                .status(StatusCode::UNAUTHORIZED)
                .body("Session expired".into())
                .unwrap());
        }

        if session_cookie.is_some() && username_cookie.is_none() {
            let mut cookie = SESSION_COOKIE.clone();
            cookie.make_removal();
            return Err(Response::builder()
                .header("Set-Cookie", cookie.encoded().to_string())
                .status(StatusCode::UNAUTHORIZED)
                .body("Invalid session".into())
                .unwrap());
        }

        jar.get(SESSION_COOKIE_NAME)
            .map(|c| SessionId(c.value().to_string()))
            .ok_or(StatusCode::UNAUTHORIZED.into_response())
    }
}

impl axum::extract::FromRequestParts<RouterState> for SessionUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        let session_id = SessionId::from_request_parts(parts, state).await?.0;
        let user = state
            .app
            .sessions
            .get(&session_id)
            .map_err(|_| StatusCode::UNAUTHORIZED.into_response())?
            .ok_or(StatusCode::UNAUTHORIZED.into_response())?;

        Ok(SessionUser(user))
    }
}

impl<T: FromRequestParts<RouterState>> axum::extract::FromRequestParts<RouterState> for MaybeExtract<T> {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &RouterState) -> Result<Self, Self::Rejection> {
        T::from_request_parts(parts, state).await.map(|su| Self(Some(su))).or_else(|_| Ok(Self(None)))
    }
}
