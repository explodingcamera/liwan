use futures_lite::StreamExt;
use oauth2::{HttpRequest, HttpResponse};

const MAX_RESPONSE_SIZE: usize = 1024 * 1024;

pub(super) async fn execute(client: &reqwest::Client, request: HttpRequest) -> Result<HttpResponse, std::io::Error> {
    let request = reqwest::Request::try_from(request).map_err(std::io::Error::other)?;
    let response = client.execute(request).await.map_err(std::io::Error::other)?;
    if response.content_length().is_some_and(|length| length > MAX_RESPONSE_SIZE as u64) {
        return Err(std::io::Error::other("provider response is too large"));
    }

    let status = response.status();
    let headers = response.headers().clone();
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(std::io::Error::other)?;
        if chunk.len() > MAX_RESPONSE_SIZE - body.len() {
            return Err(std::io::Error::other("provider response is too large"));
        }
        body.extend_from_slice(&chunk);
    }

    let mut response = http::Response::builder().status(status);
    *response.headers_mut().expect("response builder has headers") = headers;
    response.body(body).map_err(std::io::Error::other)
}
