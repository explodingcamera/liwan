#![allow(clippy::result_large_err)]

use poem::http::{Method, StatusCode, header};
use poem::web::Json;
use poem::{Endpoint, IntoResponse, Request, Response};
use poem_openapi::{ApiResponse, Object};
use rust_embed::RustEmbed;
use serde::Serialize;
use serde_json::json;
use std::{fmt::Display, marker::PhantomData};

pub async fn catch_error(err: poem::Error) -> impl IntoResponse {
    Json(json!({ "status": "error", "message": err.to_string(), "code": err.status().as_u16()}))
        .with_status(err.status())
}

#[rustfmt::skip]
pub trait PoemErrExt<T> {
    fn http_err(self, message: &str, status: StatusCode) -> poem::Result<T>;
    fn http_status(self, status: StatusCode) -> poem::Result<T>;
}

impl<T> PoemErrExt<T> for Option<T> {
    fn http_err(self, message: &str, status: StatusCode) -> poem::Result<T> {
        self.ok_or_else(|| poem::Error::from_string(message, status))
    }

    fn http_status(self, status: StatusCode) -> poem::Result<T> {
        self.ok_or_else(|| status.into())
    }
}

impl<T, E: Display> PoemErrExt<T> for Result<T, E> {
    fn http_err(self, message: &str, status: StatusCode) -> poem::Result<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(e) => {
                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    tracing::error!("{message}: {err}", err = e);
                } else {
                    tracing::debug!("{message}: {err}", err = e);
                }

                Err(poem::Error::from_string(message, status))
            }
        }
    }

    fn http_status(self, status: StatusCode) -> poem::Result<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(e) => {
                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    tracing::error!("{err}", err = e);
                } else {
                    tracing::debug!("{err}", err = e);
                }

                Err(status.into())
            }
        }
    }
}

pub struct EmbeddedFilesEndpoint<E: RustEmbed + Send + Sync>(PhantomData<E>);

impl<E: RustEmbed + Send + Sync> Default for EmbeddedFilesEndpoint<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: RustEmbed + Send + Sync> EmbeddedFilesEndpoint<E> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

pub type ApiResult<T> = poem::Result<T, poem::Error>;

#[derive(Object, Serialize)]
pub struct StatusResponse {
    status: String,
    message: Option<String>,
}

#[derive(ApiResponse)]
pub enum EmptyResponse {
    #[oai(status = 200)]
    Ok(poem_openapi::payload::Json<StatusResponse>),
}

impl EmptyResponse {
    pub fn ok() -> ApiResult<Self> {
        Ok(Self::Ok(poem_openapi::payload::Json(StatusResponse { status: "ok".to_string(), message: None })))
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

        if path.starts_with("p/") {
            let mut parts = path.splitn(3, '/').collect::<Vec<&str>>();
            parts[1] = "project";
            path = parts.join("/");
        }

        let file = if let Some(content) = E::get(&path) {
            Some(content)
        } else {
            path = format!("{path}/index.html");
            E::get(&path)
        };

        let orig_path = req.original_uri().path();
        if orig_path.ends_with('/') && file.is_some() && orig_path.len() > 1 {
            let redirect = req.original_uri().path().trim_start_matches('/').trim_end_matches('/');
            return Ok(Response::builder()
                .status(StatusCode::MOVED_PERMANENTLY)
                .header(header::LOCATION, format!("/{redirect}"))
                .body(vec![]));
        }

        match file {
            Some(content) => {
                let hash = hex::encode(content.metadata.sha256_hash());
                if req
                    .headers()
                    .get(header::IF_NONE_MATCH)
                    .is_some_and(|etag| etag.to_str().unwrap_or("000000").eq(&hash))
                {
                    return Err(StatusCode::NOT_MODIFIED.into());
                }

                // otherwise, return 200 with etag hash
                let body: Vec<u8> = content.data.into();
                let mime = mime_guess::from_path(&path).first_or_octet_stream();

                let mut builder =
                    Response::builder().header(header::CONTENT_TYPE, mime.as_ref()).header(header::ETAG, hash);

                if path.starts_with("_astro/") {
                    builder = builder.header(header::CACHE_CONTROL, "public, max-age=604800, immutable");
                }

                Ok(builder.body(body))
            }
            None => Err(StatusCode::NOT_FOUND.into()),
        }
    }
}

macro_rules! http_bail {
    ($status:expr, $message:expr) => {
        return Err(poem::Error::from_string($message, $status))
    };
    ($message:expr) => {
        return Err(poem::Error::from_string($message, poem::http::StatusCode::INTERNAL_SERVER_ERROR))
    };
}

pub(crate) use http_bail;
