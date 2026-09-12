pub mod routes;
pub mod session;
pub mod webext;

use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::handler::{Handler, HandlerWithoutStateExt};
use rust_embed::RustEmbed;

use aide::{axum::ApiRouter, openapi};
use http::{HeaderName, HeaderValue, Method, header};
use tokio::sync::{Semaphore, mpsc::Sender};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    timeout::RequestBodyDeadlineLayer,
    trace::TraceLayer,
};

use crate::app::{
    Liwan,
    models::{Event, EventExit},
};
use crate::web::webext::serve;

pub use session::MaybeSessionId;
use webext::StaticFile;

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
pub struct Files;

#[derive(RustEmbed, Clone)]
#[folder = "./tracker"]
struct Script;

#[derive(Clone)]
pub struct RouterState {
    pub app: Arc<Liwan>,
    pub events: Sender<Event>,
    pub exits: Sender<EventExit>,
    pub report_permits: Arc<Semaphore>,
}

/// Event ingestion queues used by the web server.
#[derive(Clone)]
pub struct EventQueues {
    /// Queue for normal event inserts.
    pub events: Sender<Event>,
    /// Queue for delayed event exit updates.
    pub exits: Sender<EventExit>,
}

// feTS treats directly resolved component references as circular and falls back to less precise types.
#[derive(Clone)]
struct WrapSchemaRefs;

impl schemars::transform::Transform for WrapSchemaRefs {
    fn transform(&mut self, schema: &mut schemars::Schema) {
        let already_wrapped = schema.as_object().is_some_and(|object| {
            object.len() == 1
                && object.get("anyOf").and_then(serde_json::Value::as_array).is_some_and(|any_of| {
                    any_of.len() == 1
                        && any_of[0]
                            .as_object()
                            .is_some_and(|reference| reference.len() == 1 && reference.contains_key("$ref"))
                })
        });
        if already_wrapped {
            return;
        }

        schemars::transform::transform_subschemas(self, schema);
        if let Some(reference) = schema.remove("$ref") {
            schema.insert("anyOf".to_string(), serde_json::json!([{ "$ref": reference }]));
        }
    }
}

impl Deref for RouterState {
    type Target = Arc<Liwan>;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

pub fn router(app: Arc<Liwan>, queues: EventQueues) -> Result<(axum::Router<()>, openapi::OpenApi)> {
    aide::generate::in_context(|ctx| {
        ctx.schema = ctx.schema.settings().clone().with_transform(WrapSchemaRefs).into_generator();
    });

    let mut api = openapi::OpenApi {
        info: openapi::Info { title: "Liwan API".to_string(), ..Default::default() },
        ..openapi::OpenApi::default()
    };

    let event_cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_origin(Any)
        .allow_credentials(false)
        .allow_headers([http::header::CONTENT_TYPE, http::header::ACCEPT]);

    let script_cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any)
        .allow_credentials(false)
        .allow_headers([http::header::CONTENT_TYPE, http::header::ACCEPT]);

    let set_headers = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_XSS_PROTECTION,
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self' data: 'unsafe-inline'; img-src 'self' data: https://*"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(
                "camera=(), microphone=(), geolocation=(), payment=(), usb=(), interest-cohort=()",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("same-origin"),
        ));

    let dashboard = ApiRouter::new()
        .merge(routes::admin::router())
        .merge(routes::auth::router(&app.config))
        .merge(routes::external_auth::router(&app.config))
        .merge(routes::dashboard::router());

    let router = ApiRouter::new()
        .nest("/api", routes::event::router(&app.config).layer(event_cors))
        .nest("/api/dashboard", dashboard)
        .route_service("/script.js", StaticFile::<Script>::new("script.min.js").layer(script_cors).into_service())
        .fallback(axum::routing::get(serve))
        .layer(RequestBodyDeadlineLayer::new(Duration::from_secs(30)))
        .layer(CompressionLayer::new())
        .layer(set_headers)
        .layer(TraceLayer::new_for_http())
        .with_state(RouterState {
            app: app.clone(),
            events: queues.events,
            exits: queues.exits,
            report_permits: Arc::new(Semaphore::new(app.config.limits.report_max_concurrency)),
        })
        .finish_api(&mut api);

    Ok((router, api))
}

#[cfg(debug_assertions)]
pub fn save_spec(spec: openapi::OpenApi) -> Result<()> {
    use std::path::Path;

    let path = Path::new("./web/src/api/dashboard.ts");
    if path.exists() {
        let spec = serde_json::to_string(&spec)?;

        // check if the spec has changed
        let old_spec = std::fs::read_to_string(path)?;
        if old_spec == format!("export default {spec} as const;\n") {
            return Ok(());
        }

        tracing::info!("API has changed, updating the openapi spec...");
        std::fs::write(path, format!("export default {spec} as const;\n"))?;
    }

    Ok(())
}

pub async fn start_webserver(app: Arc<Liwan>, queues: EventQueues) -> Result<()> {
    match app.onboarding.token() {
        Some(onboarding) => {
            let get_started = format!("{}/setup?t={}", app.config.base_url, onboarding);
            tracing::info!("It looks like you're running Liwan for the first time!");
            tracing::info!("You can get started by visiting: {get_started}");
            tracing::info!("To see all available commands, run `liwan --help`");
        }
        _ => {
            tracing::info!("Liwan is running on {} ({})", app.config.base_url, app.config.listen_addr());
        }
    }

    let router = router(app.clone(), queues)?;

    #[cfg(debug_assertions)]
    save_spec(router.1)?;

    let socket_addrs: Vec<_> =
        app.config.listen_addr().to_socket_addrs().context("Failed to resolve listen address")?.collect();

    let listener = tokio::net::TcpListener::bind(socket_addrs.as_slice())
        .await
        .with_context(|| format!("Failed to bind to address {}", app.config.listen_addr()))?;

    let service = router.0.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service).await.context("server exited unexpectedly")
}
