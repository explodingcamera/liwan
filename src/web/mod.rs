pub mod routes;
pub mod session;
pub mod webext;

use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, mpsc::Sender};

use anyhow::{Context, Result};
use axum::handler::{Handler, HandlerWithoutStateExt};
use rust_embed::RustEmbed;

use aide::{axum::ApiRouter, openapi};
use http::{HeaderValue, Method, header};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
};

use crate::app::{Liwan, models::Event};
use crate::web::webext::serve;

pub use session::{MaybeExtract, SessionId, SessionUser};
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
}

impl Deref for RouterState {
    type Target = Arc<Liwan>;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

pub fn router(app: Arc<Liwan>, events: Sender<Event>) -> Result<(axum::Router<()>, openapi::OpenApi)> {
    let mut api = openapi::OpenApi {
        info: openapi::Info { title: "Liwan API".to_string(), ..Default::default() },
        ..openapi::OpenApi::default()
    };

    let event_cors = CorsLayer::new().allow_methods([Method::POST]).allow_origin(Any).allow_credentials(false);
    let script_cors = CorsLayer::new().allow_methods([Method::GET]).allow_origin(Any).allow_credentials(false);

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
            header::REFERRER_POLICY,
            HeaderValue::from_static("same-origin"),
        ));

    let dashboard = ApiRouter::new()
        .merge(routes::admin::router())
        .merge(routes::auth::router())
        .merge(routes::dashboard::router());

    let router = ApiRouter::new()
        .nest("/api", routes::event::router().layer(event_cors))
        .nest("/api/dashboard", dashboard)
        .route_service("/script.js", StaticFile::<Script>::new("script.min.js").layer(script_cors).into_service())
        .fallback(axum::routing::get(serve))
        .layer(CompressionLayer::new())
        .layer(set_headers)
        .with_state(RouterState { app: app.clone(), events })
        .finish_api(&mut api);

    Ok((router, api))
}

#[cfg(debug_assertions)]
fn save_spec(spec: openapi::OpenApi) -> Result<()> {
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

pub async fn start_webserver(app: Arc<Liwan>, events: Sender<Event>) -> Result<()> {
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

    let router = router(app.clone(), events)?;

    #[cfg(debug_assertions)]
    save_spec(router.1)?;

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", app.config.port)).await.unwrap();
    let service = router.0.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service).await.context("server exited unexpectedly")
}
