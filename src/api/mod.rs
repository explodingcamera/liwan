mod auth;
mod dashboard;
mod event;
mod webext;

use crate::app::{App, Event};
use crate::config::MAX_SESSION_AGE;
use poem::web::cookie::SameSite;
use poem::web::CompressionAlgo;
use webext::*;

use crossbeam::channel::Sender;
use eyre::{Context, Result};
use rust_embed::RustEmbed;

use poem::endpoint::EmbeddedFileEndpoint;
use poem::listener::TcpListener;
use poem::middleware::{AddData, Compression, CookieJarManager};
use poem::session::{CookieConfig, ServerSession};
use poem::{post, EndpointExt, Route, Server};

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

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    println!("Starting webserver...");
    let port = app.config().port;
    let sessions = ServerSession::new(session_cookie(), app.clone());

    let api_router = Route::new()
        .at("/event", post(event::event_handler))
        .nest("/dashboard", dashboard::router().nest("/auth", auth::router()).with(sessions))
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
