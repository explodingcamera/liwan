use super::webutils::*;
use crate::app::{App, Event};
use crate::utils::hash::{hash_ip, random_visitor_id};
use crate::utils::{referer, ua};

use crossbeam::channel::Sender;
use poem::error::NotFoundError;
use poem::http::{StatusCode, Uri};
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

    let referer_val = match event.referrer.clone().map(|r| Uri::from_str(&r)) {
        // valid referer are stripped to the FQDN
        Some(Ok(referer_uri)) => {
            let referer_fqn = referer_uri.host().unwrap_or_default();
            if referer::is_spammer(referer_fqn) {
                return http_res!();
            }
            Some(referer_fqn.to_owned())
        }
        // invalid referer are kept as is (e.g. when using custom referer values outside of the browser)
        Some(Err(_)) => event.referrer.clone(),
        None => None,
    };

    let entity = app.resolve_entity(&event.entity_id).ok_or(NotFoundError)?;
    let url = Uri::from_str(&event.url).http_err("invalid url", StatusCode::BAD_REQUEST)?;
    let host = url.host().unwrap_or_default();
    let daily_salt = app.get_salt().await.http_err("internal error", StatusCode::INTERNAL_SERVER_ERROR)?;
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
        referrer: referer_val,
        visitor_id,
    };

    let _ = events.try_send(event);
    http_res!()
}
