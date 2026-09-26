use std::{
    net::IpAddr,
    sync::Arc,
    task::{Context, Poll},
};

use http::{Method, Request};
use tower::{Layer, Service};

use crate::{Client, ClientIpConfig, Event, RequestMetadata};

/// Selects incoming HTTP paths and methods that should produce analytics events.
#[derive(Debug, Clone)]
pub struct PathFilter {
    includes: Vec<String>,
    excludes: Vec<String>,
    methods: Vec<Method>,
}

impl Default for PathFilter {
    fn default() -> Self {
        Self { includes: Vec::new(), excludes: Vec::new(), methods: vec![Method::GET] }
    }
}

impl PathFilter {
    /// Creates a filter that includes every path for GET requests.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an included path prefix. No includes means every path is included.
    pub fn include(mut self, prefix: impl Into<String>) -> Self {
        self.includes.push(prefix.into());
        self
    }

    /// Adds an excluded path prefix. Excludes take precedence over includes.
    pub fn exclude(mut self, prefix: impl Into<String>) -> Self {
        self.excludes.push(prefix.into());
        self
    }

    /// Adds an HTTP method to track.
    pub fn method(mut self, method: Method) -> Self {
        if !self.methods.contains(&method) {
            self.methods.push(method);
        }
        self
    }

    fn matches(&self, method: &Method, path: &str) -> bool {
        self.methods.contains(method)
            && (self.includes.is_empty() || self.includes.iter().any(|prefix| path.starts_with(prefix)))
            && !self.excludes.iter().any(|prefix| path.starts_with(prefix))
    }
}

/// A resolved client address that upstream middleware can place in request extensions.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedClientIp(pub IpAddr);

/// Tower layer that records matching requests without delaying the wrapped service.
#[derive(Clone)]
pub struct LiwanLayer {
    client: Client,
    entity_id: Arc<str>,
    filter: PathFilter,
    origin: Option<String>,
    event_name: Arc<str>,
    client_ip: ClientIpConfig,
}

impl LiwanLayer {
    /// Creates a layer that records events for one entity.
    pub fn new(client: Client, entity_id: impl Into<Arc<str>>) -> Self {
        Self {
            client,
            entity_id: entity_id.into(),
            filter: PathFilter::default(),
            origin: None,
            event_name: "pageview".into(),
            client_ip: ClientIpConfig::default(),
        }
    }

    /// Sets the paths and methods that produce events.
    pub fn paths(mut self, filter: PathFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Sets the public origin used when incoming request URIs are relative.
    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into().trim_end_matches('/').to_string());
        self
    }

    /// Sets the event name recorded for matching requests.
    pub fn event_name(mut self, name: impl Into<Arc<str>>) -> Self {
        self.event_name = name.into();
        self
    }

    /// Configures trusted client IP header resolution.
    pub fn client_ip(mut self, config: ClientIpConfig) -> Self {
        self.client_ip = config;
        self
    }
}

impl<S> Layer<S> for LiwanLayer {
    type Service = LiwanService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LiwanService { inner, layer: self.clone() }
    }
}

/// Service produced by [`LiwanLayer`].
#[derive(Clone)]
pub struct LiwanService<S> {
    inner: S,
    layer: LiwanLayer,
}

impl<S, B> Service<Request<B>> for LiwanService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        if self.layer.filter.matches(request.method(), request.uri().path()) {
            let peer = request.extensions().get::<ResolvedClientIp>().map(|value| value.0);
            let client_ip = self.layer.client_ip.resolve(request.headers(), peer);
            let metadata = RequestMetadata::from_headers(
                request.uri(),
                request.headers(),
                client_ip,
                self.layer.origin.as_deref(),
            );
            let _ = self.layer.client.event(
                self.layer.entity_id.clone(),
                Event::new(self.layer.event_name.as_ref(), &metadata.url).with_request(metadata),
            );
        }
        self.inner.call(request)
    }
}
