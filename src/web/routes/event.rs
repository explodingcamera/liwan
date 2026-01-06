use crate::app::{Liwan, models::Event};
use crate::utils::hash::{hash_ip, visitor_id};
use crate::utils::referrer::{Referrer, process_referer};
use crate::utils::useragent;
use crate::web::RouterState;
use crate::web::webext::{ApiResult, AxumErrExt, empty_response};

use aide::axum::routing::post;
use aide::axum::{ApiRouter, IntoApiResponse};
use anyhow::{Context, Result};
use axum::Json;
use axum::extract::State;
use axum_extra::TypedHeader;
use chrono::Utc;
use http::{StatusCode, Uri};
use schemars::JsonSchema;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::sync::{Arc, LazyLock};

pub fn router() -> ApiRouter<RouterState> {
    ApiRouter::new().route("/event", post(event_handler))
}

#[derive(serde::Deserialize, JsonSchema)]
struct EventRequest {
    entity_id: String,
    name: String,
    url: String,
    referrer: Option<String>,
    utm: Option<Utm>,
}

#[derive(serde::Deserialize, JsonSchema)]
struct Utm {
    source: Option<String>,
    content: Option<String>,
    medium: Option<String>,
    campaign: Option<String>,
    term: Option<String>,
}

static EXISTING_ENTITIES: LazyLock<quick_cache::sync::Cache<String, ()>> =
    LazyLock::new(|| quick_cache::sync::Cache::new(512));

async fn event_handler(
    state: State<RouterState>,
    // RealIp(ip): RealIp, TODO
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
    Json(event): Json<EventRequest>,
) -> ApiResult<impl IntoApiResponse> {
    let url = Uri::from_str(&event.url).context("invalid url").http_err("invalid url", StatusCode::BAD_REQUEST)?;
    let app = state.app.clone();
    let events = state.events.clone();

    let ip = None; // TODO: RealIp

    // run the event processing in the background
    tokio::task::spawn_blocking(move || {
        if let Err(e) = process_event(app, events, event, url, ip, user_agent) {
            tracing::error!("Failed to process event: {:?}", e);
        }
    });

    Ok(empty_response())
}

fn process_event(
    app: Arc<Liwan>,
    events: Sender<Event>,
    event: EventRequest,
    url: Uri,
    ip: Option<IpAddr>,
    user_agent: headers::UserAgent,
) -> Result<()> {
    let referrer = match process_referer(event.referrer.as_deref()) {
        Referrer::Fqdn(fqdn) => Some(fqdn),
        Referrer::Unknown(r) => r,
        Referrer::Spammer => return Ok(()),
    };
    let referrer = referrer.map(|r| r.trim_start_matches("www.").to_string()); // remove www. prefix
    let referrer = referrer.filter(|r| r.trim().len() > 3); // ignore empty or short referrers

    if EXISTING_ENTITIES.get(&event.entity_id).is_none() {
        if !app.entities.exists(&event.entity_id).unwrap_or(false) {
            return Ok(());
        }
        EXISTING_ENTITIES.insert(event.entity_id.clone(), ());
    }

    // we delay the user agent parsing as much as possible since it's by far the most expensive operation
    let client = useragent::parse(user_agent.as_str());
    if client.is_bot() {
        return Ok(());
    }

    let visitor_id = match ip {
        Some(ip) => hash_ip(&ip, user_agent.as_str(), &app.events.get_salt()?, &event.entity_id),
        None => visitor_id(),
    };

    let (country, city) = match ip.map(|ip| app.geoip.lookup(&ip)) {
        Some(Ok(lookup)) => (lookup.country_code, lookup.city),
        _ => (None, None),
    };

    let path = url.path().to_string();
    let path = if path.len() > 1 && path.ends_with('/') { path.trim_end_matches('/').to_string() } else { path };
    let fqdn = url.host().unwrap_or_default().to_string();

    let event = Event {
        visitor_id,
        referrer,
        country,
        city,
        mobile: Some(client.is_mobile()),
        browser: client.ua_family,
        platform: client.os_family,
        created_at: Utc::now(),
        entity_id: event.entity_id,
        event: event.name,
        fqdn: fqdn.into(),
        path: path.into(),
        utm_campaign: event.utm.as_ref().and_then(|u| u.campaign.clone()),
        utm_content: event.utm.as_ref().and_then(|u| u.content.clone()),
        utm_medium: event.utm.as_ref().and_then(|u| u.medium.clone()),
        utm_source: event.utm.as_ref().and_then(|u| u.source.clone()),
        utm_term: event.utm.as_ref().and_then(|u| u.term.clone()),
    };

    events.send(event)?;
    Ok(())
}
