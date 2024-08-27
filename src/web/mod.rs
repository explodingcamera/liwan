pub mod routes;
pub mod session;
pub mod webext;

use crate::app::models::Event;
use crate::app::Liwan;
use routes::{dashboard_service, event_service};
use webext::{catch_error, EmbeddedFilesEndpoint, PoemErrExt};

pub use session::SessionUser;

use colored::Colorize;
use crossbeam::channel::Sender;
use eyre::{Context, Result};
use rust_embed::RustEmbed;

use poem::endpoint::EmbeddedFileEndpoint;
use poem::listener::TcpListener;
use poem::middleware::{AddData, Compression, CookieJarManager, Cors, SetHeader};
use poem::web::CompressionAlgo;
use poem::{EndpointExt, IntoEndpoint, Route, Server};

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
struct Files;

#[derive(RustEmbed, Clone)]
#[folder = "./tracker"]
struct Script;

#[cfg(debug_assertions)]
fn save_spec() -> Result<()> {
    use std::path::Path;

    let path = Path::new("./web/src/api/dashboard.ts");
    if path.exists() {
        let spec = serde_json::to_string(&serde_json::from_str::<serde_json::Value>(&dashboard_service().spec())?)?
            .replace(r#""servers":[],"#, "") // fets doesn't work with an empty servers array
            .replace("; charset=utf-8", "") // fets doesn't detect the json content type correctly
            .replace(r#""format":"int64","#, ""); // fets uses bigint for int64

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

pub fn create_router(app: Liwan, events: Sender<Event>) -> impl IntoEndpoint {
    let handle_events = event_service().with(Cors::new().allow_method("POST").allow_credentials(false));

    let serve_script = EmbeddedFileEndpoint::<Script>::new("script.min.js")
        .with(Cors::new().allow_method("GET").allow_credentials(false))
        .with(SetHeader::new().appending("Content-Type", "application/javascript"));

    let headers = SetHeader::new()
        .appending("X-Frame-Options", "DENY")
        .appending("X-Content-Type-Options", "nosniff")
        .appending("X-XSS-Protection", "1; mode=block")
        .appending("Content-Security-Policy", "default-src 'self' data: 'unsafe-inline'")
        .appending("Referrer-Policy", "same-origin")
        .appending("Permissions-Policy", "geolocation=(), microphone=(), camera=()");

    let api_router = Route::new()
        .nest_no_strip("/event", handle_events)
        .nest("/dashboard", dashboard_service().with(CookieJarManager::new()))
        .catch_all_error(catch_error);

    Route::new()
        .nest("/api", api_router)
        .at("/script.js", serve_script)
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .with(AddData::new(app.clone()))
        .with(AddData::new(events))
        .with(CookieJarManager::new())
        .with(Compression::new().algorithms([CompressionAlgo::BR, CompressionAlgo::GZIP]))
        .with(headers)
}

pub async fn start_webserver(app: Liwan, events: Sender<Event>) -> Result<()> {
    #[cfg(debug_assertions)]
    save_spec()?;

    let router = create_router(app.clone(), events.clone());
    let listener = TcpListener::bind(("0.0.0.0", app.config.port));

    if let Some(onboarding) = app.onboarding.token()? {
        let get_started = format!("{}/setup?t={}", app.config.base_url, onboarding).underline().bold();
        let command = "liwan --help".bold();
        tracing::info!("{}", "It looks like you're running Liwan for the first time!".white());
        tracing::info!("{}", format!("You can get started by visiting: {get_started}").white());
        tracing::info!("{}", format!("To see all available commands, run `{command}`").white());
    } else {
        tracing::info!("{}", format!("Liwan is running on {}", app.config.base_url.underline()).white());
    }

    Server::new(listener).run(router).await.wrap_err("server exited unexpectedly")
}
