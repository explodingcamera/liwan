use super::webext::*;
use crate::app::{models::Event, App};
use crate::utils::hash::{hash_ip, visitor_id};
use crate::utils::referer::process_referer;
use crate::utils::ua;

use cached::{Cached, TimedCache};
use crossbeam::channel::Sender;
use poem::http::Uri;
use poem::web::{headers, Data, RealIp, TypedHeader};
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use std::cell::RefCell;
use std::str::FromStr;

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

pub(crate) struct EventApi;
#[OpenApi]
impl EventApi {
    #[oai(path = "/event", method = "post")]
    async fn event_handler(
        &self,
        RealIp(ip): RealIp,
        Data(app): Data<&App>,
        Data(events): Data<&Sender<Event>>,
        Json(event): Json<EventRequest>,
        TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
    ) -> APIResult<EmptyResponse> {
        let client = ua::parse(user_agent.as_str());
        if ua::is_bot(&client) {
            return EmptyResponse::ok();
        }

        let Ok(referrer) = process_referer(event.referrer.as_deref()) else {
            return EmptyResponse::ok();
        };

        if !EXISTING_ENTITIES.with(|cache| cache.borrow_mut().cache_get(&event.entity_id).is_some()) {
            if !app.entity_exists(&event.entity_id).http_internal("internal error")? {
                return EmptyResponse::ok();
            }
            EXISTING_ENTITIES
                .with(|cache| cache.borrow_mut().cache_set(event.entity_id.clone(), event.entity_id.clone()));
        }

        let url = Uri::from_str(&event.url).http_bad_request("invalid url")?;
        let daily_salt = app.get_salt().await.http_internal("internal error")?;
        let visitor_id = match ip {
            Some(ip) => hash_ip(&ip, user_agent.as_str(), &daily_salt, &event.entity_id),
            None => visitor_id(),
        };

        let event = Event {
            visitor_id,
            referrer,
            city: None,
            country: None,
            browser: client.user_agent.family.to_string().into(),
            created_at: chrono::Utc::now(),
            entity_id: event.entity_id,
            event: event.name,
            fqdn: url.host().unwrap_or_default().to_string().into(),
            mobile: Some(ua::is_mobile(&client)),
            path: url.path().to_string().into(),
            platform: client.os.family.to_string().into(),
        };

        let _ = events.try_send(event);
        EmptyResponse::ok()
    }
}
