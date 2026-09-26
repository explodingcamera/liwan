use crate::{Builder, Client, Error, Transport, TransportError};

/// Reqwest-backed API transport. Reqwest TLS and other features are selected by the application.
#[derive(Clone)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// Wraps an application-configured Reqwest client.
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

impl Transport for ReqwestTransport {
    fn send(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<http::Response<()>, TransportError>> + Send + '_>> {
        Box::pin(async move {
            let (parts, body) = request.into_parts();
            let url =
                reqwest::Url::parse(&parts.uri.to_string()).map_err(|_| TransportError::new("invalid request URL"))?;
            let mut request = reqwest::Request::new(parts.method, url);
            *request.headers_mut() = parts.headers;
            *request.body_mut() = Some(body.into());
            let response =
                self.client.execute(request).await.map_err(|_| TransportError::new("HTTP request failed"))?;
            let mut result = http::Response::builder().status(response.status());
            if let Some(retry_after) = response.headers().get(http::header::RETRY_AFTER) {
                result = result.header(http::header::RETRY_AFTER, retry_after);
            }
            result.body(()).map_err(|_| TransportError::new("invalid HTTP response"))
        })
    }
}

impl Client {
    /// Starts configuring a client using a base URL or complete batch endpoint and a Reqwest client.
    pub fn builder(
        base_url: impl AsRef<str>,
        api_key: impl Into<String>,
        client: reqwest::Client,
    ) -> Result<Builder<ReqwestTransport>, Error> {
        Self::builder_with_transport(base_url, api_key, ReqwestTransport::new(client))
    }
}
