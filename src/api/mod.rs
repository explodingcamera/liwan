mod auth;
mod dashboard;
mod event;
mod session;
mod webext;

use crate::app::models::Event;
use crate::app::App;
use colored::Colorize;
use event::EventApi;
use std::path::Path;
use webext::*;

use crossbeam::channel::Sender;
use eyre::{Context, Result};
use rust_embed::RustEmbed;

use poem::endpoint::EmbeddedFileEndpoint;
use poem::listener::TcpListener;
use poem::middleware::{AddData, Compression, CookieJarManager, Cors, SetHeader};
use poem::web::CompressionAlgo;
use poem::{EndpointExt, Route, Server};
use poem_openapi::OpenApiService;

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
struct Files;

#[derive(RustEmbed, Clone)]
#[folder = "./tracker"]
struct Script;

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

pub(crate) async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    #[cfg(debug_assertions)]
    save_spec()?;

    let api_router = Route::new()
        .nest_no_strip(
            "/event",
            event_service().with(Cors::new().allow_origin("*").allow_method("POST").allow_credentials(false)),
        )
        .nest("/dashboard", dashboard_service().with(CookieJarManager::new()))
        .catch_all_error(catch_error);

    let router = Route::new()
        .nest("/api", api_router)
        .at(
            "/script.js",
            EmbeddedFileEndpoint::<Script>::new("script.min.js")
                .with(Cors::new().allow_origin("*").allow_method("GET").allow_credentials(false)),
        )
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .with(AddData::new(app.clone()))
        .with(AddData::new(events))
        .with(CookieJarManager::new())
        .with(Compression::new().algorithms([CompressionAlgo::BR, CompressionAlgo::GZIP]))
        .with(
            SetHeader::new()
                .appending("X-Frame-Options", "DENY")
                .appending("X-Content-Type-Options", "nosniff")
                .appending("X-XSS-Protection", "1; mode=block")
                .appending("Content-Security-Policy", "default-src 'self' data: 'unsafe-inline'")
                .appending("Referrer-Policy", "same-origin")
                .appending("Feature-Policy", "geolocation 'none'; microphone 'none'; camera 'none'")
                .appending("Permissions-Policy", "geolocation=(), microphone=(), camera=(), interest-cohort=()"),
        );

    let listener = TcpListener::bind(("0.0.0.0", app.config.port));

    if let Some(onboarding) = app.onboarding.read().unwrap().as_ref() {
        println!("{}", "Welcome to App!".bold().white());
        println!(
            "You can get started by visiting: {}",
            format!("http://localhost:{}/setup?t={}", app.config.port, onboarding).underline().white()
        );
        println!("{}", "To see all available commands, run `liwan --help`".bold().white());
    } else {
        println!(
            "{}",
            format!("App is running at: {}", app.config.base_url.to_string().underline().white()).bold().white()
        );
    }

    Server::new(listener).run(router).await.wrap_err("server exected unecpectedly")
}
