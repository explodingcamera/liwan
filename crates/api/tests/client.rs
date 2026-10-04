use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use http::{Request, StatusCode, header};
use liwan_api::{Client, Error, Event, Transport, TransportError};

#[derive(Clone)]
struct MockTransport {
    requests: Arc<Mutex<Vec<Request<Vec<u8>>>>>,
    statuses: Arc<Mutex<VecDeque<StatusCode>>>,
}

impl MockTransport {
    fn new(statuses: impl IntoIterator<Item = StatusCode>) -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            statuses: Arc::new(Mutex::new(statuses.into_iter().collect())),
        }
    }
}

impl Transport for MockTransport {
    fn send(
        &self,
        request: Request<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<http::Response<()>, TransportError>> + Send + '_>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request);
            let status = self.statuses.lock().unwrap().pop_front().unwrap_or(StatusCode::ACCEPTED);
            Ok(http::Response::builder().status(status).body(()).unwrap())
        })
    }
}

#[tokio::test]
async fn flushes_at_batch_size() {
    let transport = MockTransport::new([StatusCode::ACCEPTED]);
    let requests = transport.requests.clone();
    let (client, worker) = Client::builder_with_transport("https://liwan.example", "secret", transport)
        .unwrap()
        .batch_size(2)
        .build()
        .unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com/one")).unwrap();
    client.event("docs", Event::pageview("https://example.com/two")).unwrap();
    client.flush().await.unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].uri(), "https://liwan.example/api/v1/events");
    let body: serde_json::Value = serde_json::from_slice(requests[0].body()).unwrap();
    assert_eq!(body["events"].as_array().unwrap().len(), 2);
    assert_eq!(body["entityId"], "docs");
    assert_eq!(body["events"][0]["url"], "https://example.com/one");
    assert!(requests[0].headers()[header::AUTHORIZATION].is_sensitive());
}

#[tokio::test]
async fn accepts_full_batch_endpoint() {
    let transport = MockTransport::new([StatusCode::ACCEPTED]);
    let requests = transport.requests.clone();
    let (client, worker) = Client::builder_with_transport("https://liwan.example/api/v1/events/", "secret", transport)
        .unwrap()
        .build()
        .unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com")).unwrap();
    client.flush().await.unwrap();
    assert_eq!(requests.lock().unwrap()[0].uri(), "https://liwan.example/api/v1/events");
}

#[tokio::test]
async fn retries_service_unavailable() {
    let transport = MockTransport::new([StatusCode::SERVICE_UNAVAILABLE, StatusCode::ACCEPTED]);
    let requests = transport.requests.clone();
    let (client, worker) = Client::builder_with_transport("https://liwan.example", "secret", transport)
        .unwrap()
        .max_retries(1)
        .build()
        .unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com")).unwrap();
    client.flush().await.unwrap();
    assert_eq!(requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn separates_batches_by_entity() {
    let transport = MockTransport::new([StatusCode::ACCEPTED, StatusCode::ACCEPTED]);
    let requests = transport.requests.clone();
    let (client, worker) =
        Client::builder_with_transport("https://liwan.example", "secret", transport).unwrap().build().unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com/docs")).unwrap();
    client.event("shop", Event::pageview("https://example.com/shop")).unwrap();
    client.flush().await.unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    let first: serde_json::Value = serde_json::from_slice(requests[0].body()).unwrap();
    let second: serde_json::Value = serde_json::from_slice(requests[1].body()).unwrap();
    assert_eq!(first["entityId"], "docs");
    assert_eq!(second["entityId"], "shop");
}

#[tokio::test]
async fn reports_authentication_errors() {
    let transport = MockTransport::new([StatusCode::UNAUTHORIZED]);
    let (client, worker) =
        Client::builder_with_transport("https://liwan.example", "top-secret", transport).unwrap().build().unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com")).unwrap();
    let error = client.flush().await.unwrap_err();
    assert_eq!(error, Error::Authentication);
}

#[tokio::test]
async fn retries_rejected_batch_on_next_flush() {
    let transport = MockTransport::new([
        StatusCode::UNAUTHORIZED,
        StatusCode::UNAUTHORIZED,
        StatusCode::ACCEPTED,
        StatusCode::ACCEPTED,
    ]);
    let requests = transport.requests.clone();
    let (client, worker) =
        Client::builder_with_transport("https://liwan.example", "secret", transport).unwrap().build().unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com/docs")).unwrap();
    client.event("shop", Event::pageview("https://example.com/shop")).unwrap();
    assert_eq!(client.flush().await.unwrap_err(), Error::Authentication);
    client.shutdown().await.unwrap();

    let requests = requests.lock().unwrap();
    let ids = requests
        .iter()
        .map(|request| {
            serde_json::from_slice::<serde_json::Value>(request.body()).unwrap()["entityId"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, ["docs", "docs", "docs", "shop"]);
}

#[tokio::test]
async fn failed_shutdown_keeps_client_open() {
    let transport = MockTransport::new([StatusCode::UNAUTHORIZED, StatusCode::ACCEPTED, StatusCode::ACCEPTED]);
    let requests = transport.requests.clone();
    let (client, worker) =
        Client::builder_with_transport("https://liwan.example", "secret", transport).unwrap().build().unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com/one")).unwrap();
    assert_eq!(client.shutdown().await.unwrap_err(), Error::Authentication);
    client.event("docs", Event::pageview("https://example.com/two")).unwrap();
    client.shutdown().await.unwrap();

    let requests = requests.lock().unwrap();
    let delivered: serde_json::Value = serde_json::from_slice(requests[1].body()).unwrap();
    assert_eq!(delivered["events"].as_array().unwrap().len(), 2);
}

#[test]
fn rejects_zero_timeouts() {
    let transport = MockTransport::new([]);
    assert!(matches!(
        Client::builder_with_transport("https://liwan.example", "secret", transport.clone())
            .unwrap()
            .flush_interval(Duration::ZERO)
            .build(),
        Err(Error::InvalidConfiguration("flush interval must be positive"))
    ));
    assert!(matches!(
        Client::builder_with_transport("https://liwan.example", "secret", transport)
            .unwrap()
            .request_timeout(Duration::ZERO)
            .build(),
        Err(Error::InvalidConfiguration("request timeout must be positive"))
    ));
}

#[tokio::test]
async fn shutdown_drains_queued_events() {
    let transport = MockTransport::new([StatusCode::ACCEPTED]);
    let requests = transport.requests.clone();
    let (client, worker) =
        Client::builder_with_transport("https://liwan.example", "secret", transport).unwrap().build().unwrap();
    tokio::spawn(worker);

    client.event("docs", Event::pageview("https://example.com")).unwrap();
    client.shutdown().await.unwrap();
    assert_eq!(requests.lock().unwrap().len(), 1);
}
