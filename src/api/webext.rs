use std::marker::PhantomData;

use poem::{
    http::{header, Method, StatusCode},
    web::Json,
    Endpoint, IntoResponse, Request, Response,
};
use rust_embed::RustEmbed;
use serde_json::json;

pub fn get_user(session: &poem::session::Session, app: &crate::app::App) -> poem::Result<Option<crate::config::User>> {
    Ok(match session.get::<String>("username") {
        Some(username) => {
            Some(app.config().resolve_user(&username).http_err("user not found", StatusCode::UNAUTHORIZED)?)
        }
        None => None,
    })
}

pub trait PoemErrExt<T> {
    fn http_err(self, message: &str, status: StatusCode) -> poem::Result<T>;

    fn http_internal(self, message: &str) -> poem::Result<T>
    where
        Self: Sized,
    {
        self.http_err(message, StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn http_bad_request(self, message: &str) -> poem::Result<T>
    where
        Self: Sized,
    {
        self.http_err(message, StatusCode::BAD_REQUEST)
    }

    fn http_unauthorized(self, message: &str) -> poem::Result<T>
    where
        Self: Sized,
    {
        self.http_err(message, StatusCode::UNAUTHORIZED)
    }

    fn http_not_found(self, message: &str) -> poem::Result<T>
    where
        Self: Sized,
    {
        self.http_err(message, StatusCode::NOT_FOUND)
    }
}

impl<T> PoemErrExt<T> for Option<T> {
    fn http_err(self, message: &str, status: StatusCode) -> poem::Result<T> {
        self.ok_or_else(|| poem::Error::from_string(message, status))
    }
}

impl<T, E> PoemErrExt<T> for Result<T, E> {
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

#[macro_export]
macro_rules! http_bail {
    ($status:expr, $message:expr) => {
        return Err(poem::Error::from_string($message, $status))
    };
    ($message:expr) => {
        return Err(poem::Error::from_string($message, poem::http::StatusCode::INTERNAL_SERVER_ERROR))
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

pub struct EmbeddedFilesEndpoint<E: RustEmbed + Send + Sync>(PhantomData<E>);

impl<E: RustEmbed + Send + Sync> EmbeddedFilesEndpoint<E> {
    pub fn new() -> Self {
        EmbeddedFilesEndpoint(PhantomData)
    }
}

impl<E: RustEmbed + Send + Sync> Endpoint for EmbeddedFilesEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output, poem::Error> {
        let mut path = req.uri().path().trim_start_matches('/').trim_end_matches('/').to_string();
        if path.is_empty() {
            path = "index.html".to_string();
        }

        if req.method() != Method::GET {
            return Err(StatusCode::METHOD_NOT_ALLOWED.into());
        }

        let file = match E::get(&path) {
            Some(content) => Some(content),
            None => {
                path = format!("{}/index.html", path);
                E::get(&path)
            }
        };

        let orig_path = req.original_uri().path();
        if orig_path.ends_with('/') && file.is_some() && orig_path.len() > 1 {
            let redirect = req.original_uri().path().trim_start_matches('/').trim_end_matches('/');
            return Ok(Response::builder()
                .status(StatusCode::MOVED_PERMANENTLY)
                .header(header::LOCATION, format!("/{}", redirect))
                .body(vec![]));
        }

        match file {
            Some(content) => {
                let hash = hex::encode(content.metadata.sha256_hash());
                if req
                    .headers()
                    .get(header::IF_NONE_MATCH)
                    .map(|etag| etag.to_str().unwrap_or("000000").eq(&hash))
                    .unwrap_or(false)
                {
                    return Err(StatusCode::NOT_MODIFIED.into());
                }

                // otherwise, return 200 with etag hash
                let body: Vec<u8> = content.data.into();
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Ok(Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .header(header::ETAG, hash)
                    .body(body))
            }
            None => Err(StatusCode::NOT_FOUND.into()),
        }
    }
}

pub use http_bail;
pub use http_res;
