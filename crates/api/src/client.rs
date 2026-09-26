use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use async_channel::{Receiver, Sender};
use futures_lite::future;
use http::{HeaderValue, Request, StatusCode, header};
use rand::RngExt;
use serde::Serialize;
use url::Url;

use crate::{Event, Transport};

const DEFAULT_BATCH_SIZE: usize = 100;
const DEFAULT_QUEUE_CAPACITY: usize = 10_000;

/// A buffered Liwan client.
#[derive(Clone)]
pub struct Client {
    sender: Sender<Message>,
    done: Receiver<Result<(), Error>>,
    closed: Arc<AtomicBool>,
}

impl Client {
    /// Starts configuring a client with a base URL or complete batch endpoint and a custom transport.
    pub fn builder_with_transport<T: Transport>(
        base_url: impl AsRef<str>,
        api_key: impl Into<String>,
        transport: T,
    ) -> Result<Builder<T>, Error> {
        Builder::new(base_url, api_key, transport)
    }

    /// Queues an event for an entity without waiting for an HTTP request.
    pub fn event(&self, entity_id: impl Into<Arc<str>>, event: Event) -> Result<(), Error> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::Closed);
        }
        let entity_id = entity_id.into();
        if entity_id.trim().is_empty() {
            return Err(Error::InvalidConfiguration("entity ID cannot be empty"));
        }
        self.sender.try_send(Message::Event(entity_id, event)).map_err(|error| match error {
            async_channel::TrySendError::Full(_) => Error::QueueFull,
            async_channel::TrySendError::Closed(_) => Error::Closed,
        })
    }

    /// Delivers all events queued before this call.
    pub async fn flush(&self) -> Result<(), Error> {
        let (sender, receiver) = async_channel::bounded(1);
        self.sender.send(Message::Flush(sender)).await.map_err(|_| Error::Closed)?;
        receiver.recv().await.map_err(|_| Error::WorkerStopped)?
    }

    /// Stops accepting events and flushes the queue.
    pub async fn shutdown(&self) -> Result<(), Error> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Err(Error::Closed);
        }
        self.sender.close();
        self.done.recv().await.map_err(|_| Error::WorkerStopped)?
    }
}

/// Configures a buffered client.
pub struct Builder<T: Transport> {
    endpoint: Url,
    api_key: String,
    transport: T,
    batch_size: usize,
    flush_interval: Duration,
    queue_capacity: usize,
    request_timeout: Duration,
    max_retries: u32,
}

impl<T: Transport> Builder<T> {
    fn new(base_url: impl AsRef<str>, api_key: impl Into<String>, transport: T) -> Result<Self, Error> {
        let mut endpoint =
            Url::parse(base_url.as_ref()).map_err(|_| Error::InvalidConfiguration("invalid base URL"))?;
        let path = endpoint.path().trim_end_matches('/');
        let path = if path.ends_with("/api/v1/events") { path.to_owned() } else { format!("{path}/api/v1/events") };
        endpoint.set_path(&path);
        endpoint.set_query(None);
        endpoint.set_fragment(None);
        Ok(Self {
            endpoint,
            api_key: api_key.into(),
            transport,
            batch_size: DEFAULT_BATCH_SIZE,
            flush_interval: Duration::from_secs(5),
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            request_timeout: Duration::from_secs(10),
            max_retries: 4,
        })
    }

    /// Sets the number of events sent in each request, from 1 through 10,000.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Sets the maximum time an incomplete batch waits before delivery.
    pub fn flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = interval;
        self
    }

    /// Sets the bounded in-memory event capacity.
    pub fn queue_capacity(mut self, capacity: usize) -> Self {
        self.queue_capacity = capacity;
        self
    }

    /// Sets the timeout for an individual HTTP attempt.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets the maximum number of retries after the initial delivery attempt.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Builds a runtime-neutral client and its worker future.
    ///
    /// The application must spawn or poll the worker for events to be delivered.
    pub fn build(self) -> Result<(Client, Worker), Error> {
        self.validate()?;
        let (sender, receiver) = async_channel::bounded(self.queue_capacity);
        let (done_tx, done) = async_channel::bounded(1);
        let client = Client { sender, done, closed: Arc::new(AtomicBool::new(false)) };
        let worker = Worker(Box::pin(async move {
            let result = run_worker(receiver, self).await;
            let _ = done_tx.send(result).await;
        }));
        Ok((client, worker))
    }

    /// Builds the client and spawns its worker on the current Tokio runtime.
    #[cfg(feature = "tokio")]
    pub fn build_tokio(self) -> Result<Client, Error> {
        let (client, worker) = self.build()?;
        tokio::runtime::Handle::try_current()
            .map_err(|_| Error::InvalidConfiguration("a Tokio runtime is required"))?
            .spawn(worker);
        Ok(client)
    }

    fn validate(&self) -> Result<(), Error> {
        if self.batch_size == 0 || self.batch_size > 10_000 {
            return Err(Error::InvalidConfiguration("batch size must be between 1 and 10,000"));
        }
        if self.queue_capacity < self.batch_size {
            return Err(Error::InvalidConfiguration("queue capacity must be at least the batch size"));
        }
        if self.flush_interval.is_zero() {
            return Err(Error::InvalidConfiguration("flush interval must be positive"));
        }
        if self.request_timeout.is_zero() {
            return Err(Error::InvalidConfiguration("request timeout must be positive"));
        }
        if self.api_key.trim().is_empty() {
            return Err(Error::InvalidConfiguration("API key cannot be empty"));
        }
        Ok(())
    }
}

/// Runtime-neutral background work for a client.
#[must_use = "the worker must be spawned or awaited to deliver events"]
pub struct Worker(Pin<Box<dyn Future<Output = ()> + Send>>);

impl Future for Worker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.as_mut().poll(context)
    }
}

enum Message {
    Event(Arc<str>, Event),
    Flush(Sender<Result<(), Error>>),
}

async fn run_worker<T: Transport>(receiver: Receiver<Message>, config: Builder<T>) -> Result<(), Error> {
    let mut events = Vec::with_capacity(config.batch_size);
    let mut entity_id = None;
    let mut last_error = None;
    let mut deadline: Option<std::time::Instant> = None;
    loop {
        let next = if events.is_empty() {
            Some(receiver.recv().await)
        } else {
            let remaining = deadline
                .expect("non-empty batches have a deadline")
                .saturating_duration_since(std::time::Instant::now());
            future::race(async { Some(receiver.recv().await) }, async {
                futures_timer::Delay::new(remaining).await;
                None
            })
            .await
        };
        match next {
            Some(Ok(Message::Event(next_entity_id, event))) => {
                if entity_id.as_deref().is_some_and(|entity_id| entity_id != next_entity_id.as_ref())
                    && let Err(error) = send_pending(&config, &mut entity_id, &mut events).await
                {
                    last_error = Some(error);
                }
                if events.is_empty() {
                    entity_id = Some(next_entity_id);
                    deadline = Some(std::time::Instant::now() + config.flush_interval);
                }
                events.push(event);
                if events.len() >= config.batch_size {
                    if let Err(error) = send_pending(&config, &mut entity_id, &mut events).await {
                        last_error = Some(error);
                    }
                    deadline = None;
                }
            }
            Some(Ok(Message::Flush(response))) => {
                if let Err(error) = send_pending(&config, &mut entity_id, &mut events).await {
                    last_error = Some(error);
                }
                deadline = None;
                let _ = response.send(last_error.take().map_or(Ok(()), Err)).await;
            }
            Some(Err(_)) => {
                if let Err(error) = send_pending(&config, &mut entity_id, &mut events).await {
                    last_error = Some(error);
                }
                return last_error.map_or(Ok(()), Err);
            }
            None => {
                if let Err(error) = send_pending(&config, &mut entity_id, &mut events).await {
                    last_error = Some(error);
                }
                deadline = None;
            }
        }
    }
}

async fn send_pending<T: Transport>(
    config: &Builder<T>,
    entity_id: &mut Option<Arc<str>>,
    events: &mut Vec<Event>,
) -> Result<(), Error> {
    if events.is_empty() {
        return Ok(());
    }
    let result = send_batch(config, entity_id.as_deref().expect("non-empty batches have an entity ID"), events).await;
    events.clear();
    *entity_id = None;
    result
}

#[derive(Serialize)]
struct Batch<'a> {
    #[serde(rename = "entityId")]
    entity_id: &'a str,
    events: &'a [Event],
}

enum Attempt {
    Response(Result<http::Response<()>, crate::TransportError>),
    Timeout,
}

async fn send_batch<T: Transport>(config: &Builder<T>, entity_id: &str, events: &[Event]) -> Result<(), Error> {
    let body = serde_json::to_vec(&Batch { entity_id, events }).map_err(|_| Error::InvalidRequest)?;
    for attempt in 0..=config.max_retries {
        let mut authorization = HeaderValue::from_str(&format!("Bearer {}", config.api_key))
            .map_err(|_| Error::InvalidConfiguration("invalid API key"))?;
        authorization.set_sensitive(true);
        let request = Request::post(config.endpoint.as_str())
            .header(header::AUTHORIZATION, authorization)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body.clone())
            .map_err(|_| Error::InvalidRequest)?;
        let result = future::race(async { Attempt::Response(config.transport.send(request).await) }, async {
            futures_timer::Delay::new(config.request_timeout).await;
            Attempt::Timeout
        })
        .await;
        let retry_after = match result {
            Attempt::Response(Ok(response)) if response.status().is_success() => return Ok(()),
            Attempt::Response(Ok(response)) if response.status() == StatusCode::UNAUTHORIZED => {
                return Err(Error::Authentication);
            }
            Attempt::Response(Ok(response))
                if response.status() == StatusCode::BAD_REQUEST
                    || response.status() == StatusCode::PAYLOAD_TOO_LARGE =>
            {
                return Err(Error::InvalidRequest);
            }
            Attempt::Response(Ok(response))
                if response.status() == StatusCode::TOO_MANY_REQUESTS
                    || response.status() == StatusCode::SERVICE_UNAVAILABLE =>
            {
                response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs)
            }
            Attempt::Response(Ok(response)) => return Err(Error::Response(response.status())),
            Attempt::Response(Err(_)) | Attempt::Timeout => None,
        };
        if attempt < config.max_retries {
            let base = 100_u64.saturating_mul(2_u64.saturating_pow(attempt));
            let jitter = rand::rng().random_range(0..=(base / 4));
            let delay = retry_after.unwrap_or_else(|| Duration::from_millis(base + jitter));
            futures_timer::Delay::new(delay).await;
        }
    }
    Err(Error::RetriesExhausted)
}

/// A client error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("event queue is full")]
    QueueFull,
    #[error("client is closed")]
    Closed,
    #[error("Liwan rejected the API key")]
    Authentication,
    #[error("Liwan rejected the event batch")]
    InvalidRequest,
    #[error("event delivery retries were exhausted")]
    RetriesExhausted,
    #[error("event delivery worker stopped")]
    WorkerStopped,
    #[error("Liwan returned HTTP status {0}")]
    Response(StatusCode),
    #[error("{0}")]
    InvalidConfiguration(&'static str),
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::{TransportError, transport::Transport};

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
        let (client, worker) =
            Client::builder_with_transport("https://liwan.example/api/v1/events/", "secret", transport)
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
}
