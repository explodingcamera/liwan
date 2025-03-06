use crate::app::{Liwan, models::Event};
use crate::utils::hash::{hash_ip, visitor_id};
use crate::utils::referrer::{Referrer, process_referer};
use crate::utils::useragent;
use crate::web::webext::{ApiResult, EmptyResponse, PoemErrExt};

use crossbeam_channel::Sender;
use eyre::{Context, Result};
use poem::http::{StatusCode, Uri};
use poem::web::headers::UserAgent;
use poem::web::{Data, RealIp, TypedHeader, headers};
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::LazyLock;
use time::OffsetDateTime;

#[derive(Object)]
struct EventRequest {
    entity_id: String,
    name: String,
    url: String,
    referrer: Option<String>,
    utm: Option<Utm>,
}

#[derive(Object)]
struct Utm {
    source: Option<String>,
    content: Option<String>,
    medium: Option<String>,
    campaign: Option<String>,
    term: Option<String>,
}

static EXISTING_ENTITIES: LazyLock<quick_cache::sync::Cache<String, ()>> =
    LazyLock::new(|| quick_cache::sync::Cache::new(1000));

pub struct EventApi;
#[OpenApi]
impl EventApi {
    #[oai(path = "/event", method = "post")]
    async fn event_handler(
        &self,
        RealIp(ip): RealIp,
        Data(app): Data<&Liwan>,
        Data(events): Data<&Sender<Event>>,
        Json(event): Json<EventRequest>,
        TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
    ) -> ApiResult<EmptyResponse> {
        let url = Uri::from_str(&event.url).wrap_err("invalid url").http_err("invalid url", StatusCode::BAD_REQUEST)?;
        let app = app.clone();
        let events = events.clone();

        // run the event processing in the background
        tokio::task::spawn_blocking(move || {
            if let Err(e) = process_event(app, events, event, url, ip, user_agent) {
                tracing::error!("Failed to process event: {:?}", e);
            }
        });

        EmptyResponse::ok()
    }
}

fn process_event(
    app: Liwan,
    events: Sender<Event>,
    event: EventRequest,
    url: Uri,
    ip: Option<IpAddr>,
    user_agent: UserAgent,
) -> Result<()> {
    let client = useragent::parse(user_agent.as_str());
    if useragent::is_bot(&client) {
        return Ok(());
    }

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

    let visitor_id = match ip {
        Some(ip) => hash_ip(&ip, user_agent.as_str(), &app.events.get_salt()?, &event.entity_id),
        None => visitor_id(),
    };

    let (country, city) = match (&app.geoip, ip) {
        (Some(geoip), Some(ip)) => match geoip.lookup(&ip) {
            Ok(lookup) => (lookup.country_code, lookup.city),
            Err(_) => (None, None),
        },
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
        browser: client.user_agent.family.to_string().into(),
        created_at: OffsetDateTime::now_utc(),
        entity_id: event.entity_id,
        event: event.name,
        fqdn: fqdn.into(),
        path: path.into(),
        mobile: Some(useragent::is_mobile(&client)),
        platform: client.os.family.to_string().into(),
        utm_campaign: event.utm.as_ref().and_then(|u| u.campaign.clone()),
        utm_content: event.utm.as_ref().and_then(|u| u.content.clone()),
        utm_medium: event.utm.as_ref().and_then(|u| u.medium.clone()),
        utm_source: event.utm.as_ref().and_then(|u| u.source.clone()),
        utm_term: event.utm.as_ref().and_then(|u| u.term.clone()),
    };

    let _ = events.try_send(event);
    Ok(())
}
