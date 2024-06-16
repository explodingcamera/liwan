use std::str::FromStr;

use crate::app::{App, Event};
use crate::utils::hash::random_visitor_id;
use crate::utils::{hash::hash_ip, ua};
use crossbeam_channel::Sender;
use eyre::{Context, Result};
use poem::endpoint::EmbeddedFilesEndpoint;
use poem::error::NotFoundError;
use serde_json::json;

use poem::http::{StatusCode, Uri};
use poem::middleware::AddData;
use poem::web::{headers, Data, Json, RealIp, TypedHeader};
use poem::{handler, listener::TcpListener, IntoResponse, Route, Server};
use poem::{post, EndpointExt};
use rust_embed::RustEmbed;

#[derive(RustEmbed, Clone)]
#[folder = "./web/dist"]
struct Files;

#[derive(serde::Deserialize)]
struct EventRequest {
    entity_id: String,
    name: String,
    url: String,
    referrer: Option<String>,
}

#[derive(Clone)]
struct AppState {
    app: App,
    events: Sender<Event>,
}

async fn catch_error(err: poem::Error) -> impl IntoResponse {
    Json(json!({ "status": "error", "message": err.to_string() })).with_status(err.status())
}

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    println!("Starting webserver...");

    let state = AppState { app, events };
    let router = Route::new()
        .at("/api/event", post(event_handler))
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .catch_all_error(catch_error)
        .with(AddData::new(state.clone()));

    let port = state.app.config.port;
    let listener = TcpListener::bind(("0.0.0.0", port));
    println!("Listening on http://0.0.0.0:{}", port);

    Server::new(listener).run(router).await.wrap_err("server exected unecpectedly")
}

#[handler]
async fn event_handler(
    RealIp(ip): RealIp,
    Data(state): Data<&AppState>,
    Json(event): Json<EventRequest>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> poem::Result<impl IntoResponse> {
    let client = ua::parse(user_agent.as_str());
    if ua::is_bot(&client) {
        return Ok(Json(json!({ "status": "ok" })));
    }

    let entity = state.app.resolve_entity(&event.entity_id).ok_or(NotFoundError)?;
    let url =
        Uri::from_str(&event.url).map_err(|_| poem::Error::from_string("invalid url", StatusCode::BAD_REQUEST))?;
    let host = url.host().unwrap_or_default();

    let daily_salt = state.app.get_salt().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let visitor_id = match ip {
        Some(ip) => hash_ip(&ip, user_agent.as_str(), &daily_salt, &entity.id),
        None => random_visitor_id(),
    };

    let event = Event {
        browser: client.user_agent.family.to_string().into(),
        city: None,
        country: None,
        created_at: chrono::Utc::now().naive_utc(),
        entity_id: event.entity_id,
        event: event.name,
        fqdn: host.to_string().into(),
        mobile: Some(ua::is_mobile(&client)),
        path: url.path().to_string().into(),
        platform: client.os.family.to_string().into(),
        referrer: event.referrer,
        visitor_id,
    };

    let _ = state.events.try_send(event);
    Ok(Json(json!({ "status": "ok" })))
}
