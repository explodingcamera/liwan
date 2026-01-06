pub mod routes;
pub mod session;
pub mod webext;

use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, mpsc::Sender};

use anyhow::{Context, Result};
use rust_embed::RustEmbed;

use aide::{axum::ApiRouter, openapi};
use http::{Method, header};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
};

use crate::app::{Liwan, models::Event};

pub use session::SessionUser;
use webext::StaticFile;

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
struct Files;

#[derive(RustEmbed, Clone)]
#[folder = "./tracker"]
struct Script;

#[derive(Clone)]
pub struct RouterState {
    pub app: Arc<Liwan>,
    pub events: Sender<Event>,
}

impl Deref for RouterState {
    type Target = Arc<Liwan>;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

pub async fn start_webserver(app: Arc<Liwan>, events: Sender<Event>) -> Result<()> {
    let event_cors = CorsLayer::new().allow_methods([Method::POST]).allow_origin(Any).allow_credentials(false);
    let script_cors = CorsLayer::new().allow_methods([Method::GET]).allow_origin(Any).allow_credentials(false);

    let set_headers = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, "DENY"))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_XSS_PROTECTION, "1; mode=block"))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self' data: 'unsafe-inline'; img-src 'self' data: https://*",
        ))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, "same-origin"));

    let dashboard = ApiRouter::new()
        .merge(routes::admin::router())
        .merge(routes::auth::router())
        .merge(routes::dashboard::router());

    let router = ApiRouter::new()
        .nest("/api", routes::event::router().layer(event_cors))
        .nest("/api/dashboard", dashboard)
        .route_service("/script.js", StaticFile::<Script>::new("script.min.js").layer(script_cors))
        .layer(CompressionLayer::new())
        .layer(set_headers)
        .with_state(RouterState { app: app.clone(), events });

    match app.onboarding.token()? {
        Some(onboarding) => {
            let get_started = format!("{}/setup?t={}", app.config.base_url, onboarding);
            tracing::info!("It looks like you're running Liwan for the first time!");
            tracing::info!("You can get started by visiting: {get_started}");
            tracing::info!("To see all available commands, run `liwan --help`");
        }
        _ => {
            tracing::info!("Liwan is running on {}", app.config.base_url);
        }
    }

    let mut api = openapi::OpenApi::default();
    api.info = openapi::Info { description: Some("an example API".to_string()), ..openapi::Info::default() };

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", app.config.port)).await.unwrap();
    let server = router.finish_api(&mut api).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, server).await.context("server exited unexpectedly")
}
