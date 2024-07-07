mod auth;
mod dashboard;
mod event;
mod session;
mod webext;

use std::path::Path;

use crate::app::models::Event;
use crate::app::App;
use event::EventApi;
use poem::web::CompressionAlgo;
use poem_openapi::OpenApiService;
use webext::*;

use crossbeam::channel::Sender;
use eyre::{Context, Result};
use rust_embed::RustEmbed;

use poem::endpoint::EmbeddedFileEndpoint;
use poem::listener::TcpListener;
use poem::middleware::{AddData, Compression, CookieJarManager};
use poem::{EndpointExt, Route, Server};

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

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    let port = app.config().port;

    #[cfg(debug_assertions)]
    save_spec()?;

    let api_router = Route::new()
        .nest_no_strip("/event", event_service())
        .nest("/dashboard", dashboard_service().with(CookieJarManager::new()))
        .catch_all_error(catch_error);

    let router = Route::new()
        .nest("/api", api_router)
        .at("/script.js", EmbeddedFileEndpoint::<Script>::new("script.min.js"))
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .with(AddData::new(app.clone()))
        .with(AddData::new(events))
        .with(CookieJarManager::new())
        .with(Compression::new().algorithms([CompressionAlgo::BR, CompressionAlgo::GZIP]));

    let listener = TcpListener::bind(("0.0.0.0", port));

    if let Some(onboarding) = app.onboarding.read().unwrap().as_ref() {
        println!("It looks like this is your first time running liwan.");
        println!("To get started, visit http://localhost:{}/setup/{}", port, onboarding);
        println!("or run `liwan set-user <username> <password>` to create a user.");
        println!("You can change the port in the newly created liwan.config.toml file.");
    } else {
        println!("Listening on http://0.0.0.0:{}", port);
    }

    Server::new(listener).run(router).await.wrap_err("server exected unecpectedly")
}
