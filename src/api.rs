use std::net::SocketAddr;
use std::str::FromStr;

use crate::app::{App, Event};
use crate::utils::{hash::hash_ip, ua};
use axum::extract::State;
use axum::http::Uri;
use axum::{http::StatusCode, response::IntoResponse, routing::*, Json, Router};
use axum_client_ip::InsecureClientIp;
use axum_extra::{headers::UserAgent, TypedHeader};
use crossbeam_channel::Sender;
use eyre::{Context, Result};
use serde_json::json;

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

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    println!("Starting webserver...");

    let state = AppState { app, events };
    let router = Router::new()
        .route("/api/event", post(event_handler))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not found") })
        .with_state(state.clone());

    let port = state.app.config.port;
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .wrap_err_with(|| format!("failed to bind to port {}", port))?;

    println!("Listening on http://{}/", listener.local_addr()?);
    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .wrap_err("server exected unecpectedly")
}

async fn event_handler(
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    InsecureClientIp(ip): InsecureClientIp,
    State(state): State<AppState>,
    Json(event): Json<EventRequest>,
) -> impl IntoResponse {
    let daily_salt = "daily_salt";
    let url = Uri::from_str(&event.url).unwrap();
    let host = url.host().unwrap_or_default();
    let entity = state.app.resolve_entity(&event.entity_id);

    let client = ua::parse(user_agent.as_str());
    if ua::is_bot(&client) {
        return Json(json!({ "status": "ok" })).into_response();
    }

    let visitor_id = hash_ip(&ip, user_agent.as_str(), daily_salt, &event.entity_id);

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
    Json(json!({ "status": "ok" })).into_response()
}
