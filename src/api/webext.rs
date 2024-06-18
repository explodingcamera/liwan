use poem::{http::StatusCode, session::Session, web::Json, IntoResponse};
use serde_json::json;

pub trait OptionExt<T> {
    fn http_err(self, message: &'static str, status: StatusCode) -> poem::Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn http_err(self, message: &'static str, status: StatusCode) -> poem::Result<T> {
        match self {
            Some(ok) => Ok(ok),
            None => Err(poem::Error::from_string(message, status)),
        }
    }
}

pub trait ResultExt<T, E> {
    fn http_err(self, message: &str, status: StatusCode) -> poem::Result<T>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn http_err(self, message: &str, status: StatusCode) -> poem::Result<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(_) => Err(poem::Error::from_string(message, status)),
        }
    }
}

pub async fn catch_error(err: poem::Error) -> impl IntoResponse {
    Json(json!({ "status": "error", "message": err.to_string() })).with_status(err.status())
}

pub fn auth<'a>(session: &Session, app: &'a crate::app::App) -> poem::Result<&'a crate::config::User> {
    let username = session.get::<String>("username").http_err("unauthorized", StatusCode::UNAUTHORIZED)?;
    app.resolve_user(&username).http_err("user not found", StatusCode::UNAUTHORIZED)
}

#[macro_export]
macro_rules! http_bail {
    ($message:expr, $status:expr) => {
        return Err(poem::Error::from_string($message, $status))
    };
}

#[macro_export]
macro_rules! http_res {
    ($data:expr) => {
        Ok(poem::web::Json(serde_json::json!({ "status": "ok", "data": $data })))
    };
    () => {
        Ok(poem::web::Json(serde_json::json!({ "status": "ok" })))
    };
}

pub use http_bail;
pub use http_res;
