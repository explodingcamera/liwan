use crate::app::{models::Event, Liwan};
use crate::utils::hash::{hash_ip, visitor_id};
use crate::utils::referrer::{process_referer, Referrer};
use crate::utils::useragent;
use crate::web::webext::{ApiResult, EmptyResponse, PoemErrExt};

use cached::{Cached, TimedCache};
use crossbeam::channel::Sender;
use eyre::Context;
use poem::http::{StatusCode, Uri};
use poem::web::{headers, Data, RealIp, TypedHeader};
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use std::cell::RefCell;
use std::str::FromStr;
use time::OffsetDateTime;

#[derive(Object)]
struct EventRequest {
    entity_id: String,
    name: String,
    url: String,
    referrer: Option<String>,
}

thread_local! {
    // Cache for existing entities to reject invalid ones
    static EXISTING_ENTITIES: RefCell<TimedCache<String, String>> = TimedCache::with_lifespan(60 * 60).into(); // 1 hour
}

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
        let client = useragent::parse(user_agent.as_str());
        if useragent::is_bot(&client) {
            return EmptyResponse::ok();
        }

        let referrer = match process_referer(event.referrer.as_deref()) {
            Referrer::Fqdn(fqdn) => Some(fqdn),
            Referrer::Unknown(r) => r,
            Referrer::Spammer => return EmptyResponse::ok(),
        };

        let referrer = referrer.map(|r| r.trim_start_matches("www.").to_string()); // remove www. prefix
        let referrer = referrer.filter(|r| r.trim().len() > 3); // ignore empty or short referrers

        if !EXISTING_ENTITIES.with(|cache| cache.borrow_mut().cache_get(&event.entity_id).is_some()) {
            if !app.entities.exists(&event.entity_id).http_status(StatusCode::INTERNAL_SERVER_ERROR)? {
                return EmptyResponse::ok();
            }
            EXISTING_ENTITIES
                .with(|cache| cache.borrow_mut().cache_set(event.entity_id.clone(), event.entity_id.clone()));
        }

        let url = Uri::from_str(&event.url).wrap_err("invalid url").http_err("invalid url", StatusCode::BAD_REQUEST)?;
        let daily_salt = app.events.get_salt().await.http_status(StatusCode::INTERNAL_SERVER_ERROR)?;
        let visitor_id = match ip {
            Some(ip) => hash_ip(&ip, user_agent.as_str(), &daily_salt, &event.entity_id),
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
        };

        let _ = events.try_send(event);
        EmptyResponse::ok()
    }
}
