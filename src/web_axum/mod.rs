pub mod routes;
pub mod session;
pub mod webext;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::app::models::Event;
use crate::{app::Liwan, web_axum::webext::StaticFile};
use axum::Extension;
use axum::handler::Handler;
use axum::http::{Method, header};
use axum::response::IntoResponse;
use axum::{Router, error_handling::HandleErrorLayer, http::StatusCode, routing::IntoMakeService};
use routes::{dashboard_service, event_service};
use std::sync::mpsc::Sender;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use aide::{
    axum::{
        ApiRouter, IntoApiResponse,
        routing::{get, post},
    },
    openapi::{Info, OpenApi},
};

pub use session::SessionUser;

use anyhow::{Context, Result};
use rust_embed::RustEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
struct Files;

#[derive(RustEmbed, Clone)]
#[folder = "./tracker"]
struct Script;

#[derive(Clone)]
pub struct State {
    pub app: Arc<Liwan>,
    pub events: Sender<Event>,
}

pub async fn start_webserver(app: Arc<Liwan>, events: Sender<Event>) -> Result<()> {
    let router = Router::new();

    let event_cors = CorsLayer::new().allow_methods([Method::POST]).allow_origin(Any).allow_credentials(false);
    let script_cors = CorsLayer::new().allow_methods([Method::GET]).allow_origin(Any).allow_credentials(false);

    let set_headers = ServiceBuilder::new()
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
        .nest("/api/event", routes::event::router().layer(event_cors))
        .nest("/api/dashboard", dashboard)
        .route_service("/script.js", StaticFile::<Script>::new("script.min.js").layer(script_cors))
        .layer(CompressionLayer::new())
        .layer(set_headers)
        .with_state(State { app: app.clone(), events });

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

    let mut api = OpenApi::default();
    api.info = Info { description: Some("an example API".to_string()), ..Info::default() };

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", app.config.port)).await.unwrap();
    let server = router.finish_api(&mut api).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, server).await.context("server exited unexpectedly")
}
