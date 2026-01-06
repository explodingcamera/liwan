use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::Extension,
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::cookie::CookieJar;

use crate::app::models::User;

pub const MAX_SESSION_AGE: Duration = Duration::from_secs(24 * 60 * 60 * 14);

pub static PUBLIC_COOKIE_NAME: &str = "liwan-username";
pub static SESSION_COOKIE_NAME: &str = "liwan-session";

#[derive(Debug, Clone)]
pub struct SessionId(pub String);

#[derive(Debug, Clone)]
pub struct SessionUser(pub User);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for SessionId {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        jar.get(SESSION_COOKIE_NAME).map(|c| SessionId(c.value().to_string())).ok_or(StatusCode::UNAUTHORIZED)
    }
}

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for SessionUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let session_id = SessionId::from_request_parts(parts, _state).await?.0;

        let Extension(app): Extension<Arc<crate::app::Liwan>> =
            Extension::from_request_parts(parts, _state).await.map_err(|_| StatusCode::UNAUTHORIZED)?;

        let user =
            app.sessions.get(&session_id).map_err(|_| StatusCode::UNAUTHORIZED)?.ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(SessionUser(user))
    }
}
