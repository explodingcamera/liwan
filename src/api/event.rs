use super::webext::*;
use crate::app::{App, Event};
use crate::utils::hash::{hash_ip, random_visitor_id};
use crate::utils::referer::process_referer;
use crate::utils::ua;

use crossbeam::channel::Sender;
use poem::error::NotFoundError;
use poem::http::Uri;
use poem::web::{headers, Data, Json, RealIp, TypedHeader};
use poem::{handler, IntoResponse};
use std::str::FromStr;

#[derive(serde::Deserialize)]
struct EventRequest {
    entity_id: String,
    name: String,
    url: String,
    referrer: Option<String>,
}

#[handler]
pub(super) async fn event_handler(
    RealIp(ip): RealIp,
    Data(app): Data<&App>,
    Data(events): Data<&Sender<Event>>,
    Json(event): Json<EventRequest>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> poem::Result<impl IntoResponse> {
    let client = ua::parse(user_agent.as_str());
    if ua::is_bot(&client) {
        return http_res!();
    }

    let Ok(referrer) = process_referer(event.referrer.as_deref()) else {
        return http_res!();
    };

    let entity = app.config().resolve_entity(&event.entity_id).ok_or(NotFoundError)?;
    let url = Uri::from_str(&event.url).http_bad_request("invalid url")?;
    let daily_salt = app.get_salt().await.http_internal("internal error")?;
    let visitor_id = match ip {
        Some(ip) => hash_ip(&ip, user_agent.as_str(), &daily_salt, &entity.id),
        None => random_visitor_id(),
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
    http_res!()
}
