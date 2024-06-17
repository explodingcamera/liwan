use std::str::FromStr;
use std::time::Duration;

use crate::app::{App, Event};
use crate::utils::hash::random_visitor_id;
use crate::utils::{hash::hash_ip, ua};
use crossbeam::channel::Sender;
use eyre::{Context, Result};
use poem::web::cookie::{Cookie, CookieJar};
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::json;

use poem::endpoint::EmbeddedFilesEndpoint;
use poem::error::NotFoundError;
use poem::http::{header, StatusCode, Uri};
use poem::middleware::{AddData, CookieJarManager};
use poem::session::{CookieConfig, ServerSession, Session};
use poem::web::{headers, Data, Form, Json, RealIp, TypedHeader};
use poem::{handler, listener::TcpListener, post, EndpointExt, IntoResponse, Response, Route, Server};

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

const MAX_SESSION_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 14);

pub async fn start_webserver(app: App, events: Sender<Event>) -> Result<()> {
    println!("Starting webserver...");
    let port = app.config.port;
    let sessions = ServerSession::new(
        CookieConfig::default().max_age(Some(MAX_SESSION_AGE)).name("liwan-session").secure(false),
        app.clone(),
    );

    let api_router =
        Route::new().at("/login", post(login_handler)).at("/logout", post(logout_handler)).with(sessions);

    let router = Route::new()
        .at("/api/event", post(event_handler))
        .nest("/api/private", api_router)
        .nest("/", EmbeddedFilesEndpoint::<Files>::new())
        .catch_all_error(catch_error)
        .with(CookieJarManager::new())
        .with(AddData::new(AppState { app: app.clone(), events }));

    let listener = TcpListener::bind(("0.0.0.0", port));
    println!("Listening on http://0.0.0.0:{}", port);

    Server::new(listener).run(router).await.wrap_err("server exected unecpectedly")
}

#[derive(Deserialize)]
struct LoginParams {
    username: String,
    password: String,
}

#[handler]
async fn login_handler(
    Data(state): Data<&AppState>,
    Json(params): Json<LoginParams>,
    session: &Session,
    cookies: &CookieJar,
) -> poem::Result<impl IntoResponse> {
    session.purge();
    if !state.app.check_login(&params.username, &params.password) {
        return Err(poem::Error::from_string("invalid username or password", StatusCode::UNAUTHORIZED));
    }

    let mut public_cookie = Cookie::new_with_str("username", params.username.clone());
    public_cookie.set_max_age(MAX_SESSION_AGE);
    cookies.add(public_cookie);

    session.set("username", params.username);
    Ok(Json(json!({ "status": "ok" })))
}

#[handler]
async fn logout_handler(session: &Session, cookies: &CookieJar) -> poem::Result<impl IntoResponse> {
    session.purge();
    cookies.remove("username");
    Ok(Response::builder().status(StatusCode::FOUND).header(header::LOCATION, "/login").finish())
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
    let url = Uri::from_str(&event.url)
        .map_err(|_| poem::Error::from_string("invalid url", StatusCode::BAD_REQUEST))?;
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
        created_at: chrono::Utc::now(),
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
