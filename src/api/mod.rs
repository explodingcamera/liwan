mod dashboard;
mod event;
mod webutils;

use crate::app::{App, Event};
use crate::config::MAX_SESSION_AGE;
use webutils::*;

use crossbeam::channel::Sender;
use eyre::{Context, Result};
use rust_embed::RustEmbed;

use poem::endpoint::EmbeddedFilesEndpoint;
use poem::listener::TcpListener;
use poem::middleware::{AddData, CookieJarManager};
use poem::session::{CookieConfig, ServerSession};
use poem::{post, EndpointExt, Route, Server};

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
struct Files;

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    println!("Starting webserver...");
    let port = app.config.port;
    let sessions = ServerSession::new(
        CookieConfig::default().max_age(Some(MAX_SESSION_AGE)).name("liwan-session").secure(false),
        app.clone(),
    );

    let private_router = Route::new()
        .at("/login", post(dashboard::login_handler))
        .at("/logout", post(dashboard::logout_handler))
        .at("/groups", post(dashboard::groups_handler))
        .with(sessions);

    let router = Route::new()
        .at("/api/event", post(event::event_handler))
        .nest("/api/private", private_router)
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .catch_all_error(catch_error)
        .with(CookieJarManager::new())
        .with(AddData::new(app))
        .with(AddData::new(events));

    let listener = TcpListener::bind(("0.0.0.0", port));
    println!("Listening on http://0.0.0.0:{}", port);

    Server::new(listener).run(router).await.wrap_err("server exected unecpectedly")
}
