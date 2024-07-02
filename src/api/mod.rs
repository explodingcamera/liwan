mod auth;
mod dashboard;
mod event;
mod webext;

use std::path::Path;

use crate::app::{App, Event};
use crate::config::MAX_SESSION_AGE;
use event::EventApi;
use poem::web::cookie::SameSite;
use poem::web::CompressionAlgo;
use poem_openapi::OpenApiService;
use webext::*;

use crossbeam::channel::Sender;
use eyre::{Context, Result};
use rust_embed::RustEmbed;

use poem::endpoint::EmbeddedFileEndpoint;
use poem::listener::TcpListener;
use poem::middleware::{AddData, Compression, CookieJarManager};
use poem::session::{CookieConfig, ServerSession};
use poem::{EndpointExt, Route, Server};

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
struct Files;

#[derive(RustEmbed, Clone)]
#[folder = "./tracker"]
struct Script;

pub fn session_cookie() -> CookieConfig {
    CookieConfig::default()
        .max_age(Some(MAX_SESSION_AGE))
        .name("liwan-session")
        .same_site(SameSite::Strict)
        .secure(false)
        .path("/api/dashboard")
}

fn event_service() -> OpenApiService<EventApi, ()> {
    OpenApiService::new(event::EventApi, "event api", "1.0").url_prefix("/api/")
}

fn dashboard_service() -> OpenApiService<(dashboard::DashboardAPI, auth::AuthApi), ()> {
    OpenApiService::new((dashboard::DashboardAPI, auth::AuthApi), "dashboard api", "1.0").url_prefix("/api/dashboard")
}

fn save_spec() -> Result<()> {
    let path = Path::new("./web/src/api/dashboard.ts");
    if path.exists() {
        let spec = serde_json::to_string(&serde_json::from_str::<serde_json::Value>(&dashboard_service().spec())?)?
            .replace(r#""servers":[],"#, "") // fets doesn't work with an empty servers array
            .replace("; charset=utf-8", "") // fets doesn't detect the json content type correctly
            .replace(r#""format":"int64","#, ""); // fets uses bigint for int64

        std::fs::write(path, format!("export default {} as const;\n", spec))?;
    }
    Ok(())
}

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    println!("Starting webserver...");
    let port = app.config().port;
    let sessions = ServerSession::new(session_cookie(), app.clone());

    #[cfg(debug_assertions)]
    save_spec()?;

    let api_router = Route::new()
        .nest_no_strip("/event", event_service())
        .nest("/dashboard", dashboard_service().with(sessions))
        .catch_all_error(catch_error);

    let router = Route::new()
        .nest("/api", api_router)
        .at("/script.js", EmbeddedFileEndpoint::<Script>::new("script.min.js"))
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .with(AddData::new(app))
        .with(AddData::new(events))
        .with(CookieJarManager::new())
        .with(Compression::new().algorithms([CompressionAlgo::BR, CompressionAlgo::GZIP]));

    let listener = TcpListener::bind(("0.0.0.0", port));
    println!("Listening on http://0.0.0.0:{}", port);

    Server::new(listener).run(router).await.wrap_err("server exected unecpectedly")
}
