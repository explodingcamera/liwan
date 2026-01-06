#![allow(clippy::result_large_err)]

use std::convert::Infallible;
use std::fmt::Display;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use aide::OperationOutput;
use aide::axum::IntoApiResponse;
use axum::body::{Body, Bytes};
use axum::response::IntoResponse;
use axum::{Json, extract};
use http::{Request, Response, StatusCode, header};
use rust_embed::RustEmbed;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::json;
use tower::Service;

pub type ApiResult<T, E = ApiError> = Result<T, E>;

#[rustfmt::skip]
pub trait AxumErrExt<T> {
    fn http_err(self, message: &str, status: StatusCode) -> ApiResult<T>;
    fn http_status(self, status: StatusCode) -> ApiResult<T>;
}

impl<T> AxumErrExt<T> for Option<T> {
    fn http_err(self, message: &str, status: StatusCode) -> ApiResult<T> {
        self.ok_or_else(|| ApiError { message: message.to_string(), status })
    }

    fn http_status(self, status: StatusCode) -> ApiResult<T> {
        self.ok_or_else(|| status.into())
    }
}

impl<T, E: Display> AxumErrExt<T> for Result<T, E> {
    fn http_err(self, message: &str, status: StatusCode) -> ApiResult<T> {
        match self {
            Ok(ok) => Ok(ok),
            Err(e) => {
                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    tracing::error!("{message}: {err}", err = e);
                } else {
                    tracing::debug!("{message}: {err}", err = e);
                }
                Err(ApiError { message: message.to_string(), status })
            }
        }
    }

    fn http_status(self, status: StatusCode) -> ApiResult<T> {
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

pub struct ApiError {
    pub message: String,
    pub status: StatusCode,
}

impl OperationOutput for ApiError {
    type Inner = Self;
}

impl From<StatusCode> for ApiError {
    fn from(status: StatusCode) -> Self {
        ApiError { message: status.canonical_reason().unwrap_or("Unknown error").to_string(), status }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> http::Response<Body> {
        let body = Json(
            json!({ "status": self.status.canonical_reason(), "message": self.message, "code": self.status.as_u16() }),
        );
        (self.status, body).into_response()
    }
}

pub async fn call<E: RustEmbed + Send + Sync>(
    orig_uri: extract::OriginalUri,
    req: Request<Bytes>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut path = req.uri().path().trim_start_matches('/').trim_end_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    if req.method() != http::Method::GET {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
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

    let orig_path = orig_uri.path();
    if orig_path.ends_with('/') && file.is_some() && orig_path.len() > 1 {
        let redirect = orig_uri.path().trim_start_matches('/').trim_end_matches('/');
        return Ok(Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header(header::LOCATION, format!("/{redirect}"))
            .body(Body::empty())
            .unwrap());
    }

    let Some(content) = file else { return Err(StatusCode::NOT_FOUND) };

    let hash = hex::encode(content.metadata.sha256_hash());
    if let Some(etag) = req.headers().get(header::IF_NONE_MATCH)
        && etag.to_str().unwrap_or("000000") == hash
    {
        return Err(StatusCode::NOT_MODIFIED);
    }

    let body = Body::from(content.data);
    let mime = content.metadata.mimetype();

    let mut builder = Response::builder().header(header::CONTENT_TYPE, mime).header(header::ETAG, hash);

    if path.starts_with("_astro/") {
        builder = builder.header(header::CACHE_CONTROL, "public, max-age=604800, immutable");
    }

    Ok(builder.body(body).unwrap())
}

#[derive(Clone)]
pub struct StaticFile<T>(&'static str, PhantomData<T>);

impl<T> StaticFile<T> {
    pub const fn new(file_path: &'static str) -> Self {
        StaticFile(file_path, PhantomData)
    }
}

impl<T: RustEmbed + Send + Sync> IntoResponse for StaticFile<T> {
    fn into_response(self) -> http::Response<Body> {
        match T::get(self.0) {
            Some(content) => ([(header::CONTENT_TYPE, content.metadata.mimetype())], content.data).into_response(),
            None => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

impl<T: RustEmbed + Send + Sync> Service<Request<Body>> for StaticFile<T> {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        Box::pin(async {
            Ok(match T::get(self.0) {
                Some(content) => Response::builder()
                    .header(header::CONTENT_TYPE, content.metadata.mimetype())
                    .body(Body::from(content.data))
                    .expect("failed to build response"),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .expect("failed to build response"),
            })
        })
    }
}

pub(crate) fn empty_response() -> impl IntoApiResponse {
    #[derive(Serialize, JsonSchema)]
    struct StatusResponse {
        status: String,
    }

    (StatusCode::OK, Json(StatusResponse { status: "OK".into() }))
}

macro_rules! http_bail {
    ($status:expr, $($arg:tt)*) => {
        return Err(crate::web::webext::ApiError {
            message: format!($($arg)*),
            status: $status,
        })
    };
}

pub(crate) use http_bail;
