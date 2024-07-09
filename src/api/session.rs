use std::time::Duration;

use lazy_static::lazy_static;
use poem::{
    web::cookie::{Cookie, SameSite},
    FromRequest,
};

use crate::app::models::User;

use super::PoemErrExt;
pub(crate) const MAX_SESSION_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 14);

lazy_static! {
    pub(crate) static ref PUBLIC_COOKIE: Cookie = {
        let mut public_cookie = Cookie::named("liwan-username");
        public_cookie.set_max_age(MAX_SESSION_AGE);
        public_cookie.set_http_only(false);
        public_cookie.set_path("/");
        public_cookie.set_same_site(SameSite::Strict);
        public_cookie
    };
    pub(crate) static ref SESSION_COOKIE: Cookie = {
        let mut session_cookie = Cookie::named("liwan-session");
        session_cookie.set_max_age(MAX_SESSION_AGE);
        session_cookie.set_http_only(true);
        session_cookie.set_path("/api/dashboard");
        session_cookie.set_same_site(SameSite::Strict);
        session_cookie
    };
}

pub(crate) struct SessionId(pub(crate) String);
pub(crate) struct SessionUser(pub(crate) User);

impl<'a> FromRequest<'a> for SessionId {
    async fn from_request(req: &'a poem::Request, _body: &mut poem::RequestBody) -> poem::Result<Self> {
        let session_id = req.cookie().get(SESSION_COOKIE.name()).map(|cookie| cookie.value_str().to_owned());
        let session_id = session_id.http_unauthorized("Unauthorized")?;
        Ok(Self(session_id))
    }
}

impl<'a> FromRequest<'a> for SessionUser {
    async fn from_request(req: &'a poem::Request, _body: &mut poem::RequestBody) -> poem::Result<Self> {
        let session_id = SessionId::from_request(req, _body).await?.0;
        let app = req.data::<crate::app::App>().http_internal("Internal error")?;
        let username =
            app.session_get(&session_id).http_unauthorized("Unauthorized")?.http_unauthorized("Unauthorized")?;
        let user = app.user(&username).http_internal("Internal error")?;
        Ok(Self(user))
    }
}
