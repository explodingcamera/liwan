mod auth;
mod dashboard;
mod event;
mod webext;

use crate::app::{App, Event};
use crate::config::MAX_SESSION_AGE;
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

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    println!("Starting webserver...");
    let port = app.config().port;
    let sessions = ServerSession::new(
        CookieConfig::default().max_age(Some(MAX_SESSION_AGE)).name("liwan-session").secure(false),
        app.clone(),
    );

    let dashboard_router =
        Route::new().nest("/auth", auth::router()).at("/groups", post(dashboard::groups_handler)).with(sessions);

    let router = Route::new()
        .at("/api/event", post(event::event_handler))
        .nest("/api/dashboard", dashboard_router)
        .at("/script.js", EmbeddedFileEndpoint::<Script>::new("script.min.js"))
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .catch_all_error(catch_error)
        .with(AddData::new(app))
        .with(AddData::new(events))
        .with(CookieJarManager::new())
        .with(Compression::new().algorithms([CompressionAlgo::BR, CompressionAlgo::GZIP]));

    let listener = TcpListener::bind(("0.0.0.0", port));
    println!("Listening on http://0.0.0.0:{}", port);

    Server::new(listener).run(router).await.wrap_err("server exected unecpectedly")
}
