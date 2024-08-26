use std::sync::LazyLock;
use std::time::Duration;

use crate::app::models::User;
use poem::web::cookie::{Cookie, SameSite};
use poem::FromRequest;

pub const MAX_SESSION_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 14);

pub static PUBLIC_COOKIE: LazyLock<Cookie> = LazyLock::new(|| {
    let mut public_cookie = Cookie::named("liwan-username");
    public_cookie.set_max_age(MAX_SESSION_AGE);
    public_cookie.set_http_only(false);
    public_cookie.set_path("/");
    public_cookie.set_same_site(SameSite::Strict);
    public_cookie
});

pub static SESSION_COOKIE: LazyLock<Cookie> = LazyLock::new(|| {
    let mut session_cookie = Cookie::named("liwan-session");
    session_cookie.set_max_age(MAX_SESSION_AGE);
    session_cookie.set_http_only(true);
    session_cookie.set_path("/api/dashboard");
    session_cookie.set_same_site(SameSite::Strict);
    session_cookie
});

pub struct SessionId(pub String);
pub struct SessionUser(pub User);

impl<'a> FromRequest<'a> for SessionId {
    async fn from_request(req: &'a poem::Request, _body: &mut poem::RequestBody) -> poem::Result<Self> {
        let session_id = req.cookie().get(SESSION_COOKIE.name()).map(|cookie| cookie.value_str().to_owned());
        let session_id = session_id.ok_or_else(|| poem::Error::from_status(poem::http::StatusCode::UNAUTHORIZED))?;

        Ok(Self(session_id))
    }
}

impl<'a> FromRequest<'a> for SessionUser {
    async fn from_request(req: &'a poem::Request, body: &mut poem::RequestBody) -> poem::Result<Self> {
        let session_id = SessionId::from_request(req, body).await?.0;
        let app = req
            .data::<crate::app::Liwan>()
            .ok_or_else(|| poem::Error::from_status(poem::http::StatusCode::UNAUTHORIZED))?;

        let username = app
            .sessions
            .get(&session_id)
            .map_err(|_| poem::Error::from_status(poem::http::StatusCode::UNAUTHORIZED))?
            .ok_or_else(|| poem::Error::from_status(poem::http::StatusCode::UNAUTHORIZED))?;

        let user =
            app.users.get(&username).map_err(|_| poem::Error::from_status(poem::http::StatusCode::UNAUTHORIZED))?;

        Ok(Self(user))
    }
}
