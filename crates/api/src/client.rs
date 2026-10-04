use std::{
    collections::VecDeque,
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
use http::{HeaderValue, Request, StatusCode, Uri, header};
use serde::Serialize;

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
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::Closed);
        }
        self.flush().await?;
        if self.closed.swap(true, Ordering::AcqRel) {
            return Err(Error::Closed);
        }
        self.sender.close();
        self.done.recv().await.map_err(|_| Error::WorkerStopped)?
    }
}

/// Configures a buffered client.
pub struct Builder<T: Transport> {
    endpoint: Uri,
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
        let base: Uri = base_url.as_ref().parse().map_err(|_| Error::InvalidConfiguration("invalid base URL"))?;
        let (Some(scheme), Some(authority)) = (base.scheme(), base.authority()) else {
            return Err(Error::InvalidConfiguration("invalid base URL"));
        };
        let path = base.path().trim_end_matches('/');
        let path = if path.ends_with("/api/v1/events") { path.to_owned() } else { format!("{path}/api/v1/events") };
        let endpoint = Uri::builder()
            .scheme(scheme.clone())
            .authority(authority.clone())
            .path_and_query(path)
            .build()
            .map_err(|_| Error::InvalidConfiguration("invalid base URL"))?;
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

#[allow(clippy::large_enum_variant)]
enum Message {
    Event(Arc<str>, Event),
    Flush(Sender<Result<(), Error>>),
}

async fn run_worker<T: Transport>(receiver: Receiver<Message>, config: Builder<T>) -> Result<(), Error> {
    let mut events = VecDeque::with_capacity(config.batch_size);
    let mut deadline: Option<std::time::Instant> = None;
    loop {
        let next = if events.is_empty() {
            Some(receiver.recv().await)
        } else {
            let remaining = deadline
                .expect("non-empty batches have a deadline")
                .saturating_duration_since(std::time::Instant::now());
            if events.len() >= config.queue_capacity {
                futures_timer::Delay::new(remaining).await;
                None
            } else {
                future::race(async { Some(receiver.recv().await) }, async {
                    futures_timer::Delay::new(remaining).await;
                    None
                })
                .await
            }
        };
        match next {
            Some(Ok(Message::Event(next_entity_id, event))) => {
                if events.is_empty() {
                    deadline = Some(std::time::Instant::now() + config.flush_interval);
                }
                events.push_back((next_entity_id, event));
                if events.len() >= config.batch_size
                    || events.front().is_some_and(|(id, _)| id != &events.back().unwrap().0)
                {
                    let _ = send_pending(&config, &mut events).await;
                    deadline = (!events.is_empty()).then(|| std::time::Instant::now() + config.flush_interval);
                }
            }
            Some(Ok(Message::Flush(response))) => {
                let mut result = Ok(());
                while !events.is_empty() {
                    if let Err(error) = send_pending(&config, &mut events).await {
                        result = Err(error);
                        break;
                    }
                }
                deadline = (!events.is_empty()).then(|| std::time::Instant::now() + config.flush_interval);
                let _ = response.send(result).await;
            }
            Some(Err(_)) => {
                while !events.is_empty() {
                    send_pending(&config, &mut events).await?;
                }
                return Ok(());
            }
            None => {
                let _ = send_pending(&config, &mut events).await;
                deadline = (!events.is_empty()).then(|| std::time::Instant::now() + config.flush_interval);
            }
        }
    }
}

async fn send_pending<T: Transport>(
    config: &Builder<T>,
    events: &mut VecDeque<(Arc<str>, Event)>,
) -> Result<(), Error> {
    let Some((entity_id, _)) = events.front() else {
        return Ok(());
    };
    let batch = events
        .iter()
        .take_while(|(id, _)| id == entity_id)
        .take(config.batch_size)
        .map(|(_, event)| event.clone())
        .collect::<Vec<_>>();
    send_batch(config, entity_id, &batch).await?;
    events.drain(..batch.len());
    Ok(())
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
        let request = Request::post(config.endpoint.clone())
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
            let backoff = Duration::from_millis(100_u64.saturating_mul(2_u64.saturating_pow(attempt)));
            let delay = retry_after.unwrap_or(backoff);
            futures_timer::Delay::new(delay).await;
        }
    }
    Err(Error::RetriesExhausted)
}

/// A client error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The event queue has reached its capacity.
    #[error("event queue is full")]
    QueueFull,
    /// The client is no longer accepting events.
    #[error("client is closed")]
    Closed,
    /// Liwan rejected the API key.
    #[error("Liwan rejected the API key")]
    Authentication,
    /// Liwan rejected the event batch.
    #[error("Liwan rejected the event batch")]
    InvalidRequest,
    /// Delivery failed after all retries.
    #[error("event delivery retries were exhausted")]
    RetriesExhausted,
    /// The background worker stopped before completing the operation.
    #[error("event delivery worker stopped")]
    WorkerStopped,
    /// Liwan returned an unexpected HTTP status.
    #[error("Liwan returned HTTP status {0}")]
    Response(StatusCode),
    /// A client setting is invalid.
    #[error("{0}")]
    InvalidConfiguration(&'static str),
}
