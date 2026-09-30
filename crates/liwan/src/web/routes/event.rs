use crate::app::models::{
    EventExit, EventProperties, FilterType, GeoDetail, IngestDropRule, IngestFilter, ResolvedCollectionSettings,
    VisitorGroupMode, hostname_allowed,
};
use crate::app::{Liwan, models::Event};
use crate::config::Config;
use crate::utils::hash::{visitor_group_id, visitor_group_id_cidr, visitor_group_id_fallback};
use crate::utils::referrer::{Referrer, process_referer};
use crate::utils::useragent;
use crate::web::RouterState;
use crate::web::webext::{
    ApiResult, AxumErrExt, ClientIp, ClientIpKeyExtractor, GeoLocationHeaders, empty_response, http_bail,
};

use aide::axum::routing::post;
use aide::axum::{ApiRouter, IntoApiResponse};
use anyhow::{Context, Result};
use axum::body::Bytes;
use axum::extract::State;
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use http::StatusCode;
use schemars::JsonSchema;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::cors::{Any, CorsLayer};
use url::Url;

pub fn router(config: &Config) -> ApiRouter<RouterState> {
    let limiter = GovernorConfigBuilder::default()
        .per_second(2)
        .burst_size(10)
        .key_extractor(ClientIpKeyExtractor::new(config))
        .finish()
        .expect("valid governor config");
    let governor_limiter = limiter.limiter().clone();

    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            governor_limiter.retain_recent();
        }
    });

    let browser_cors = CorsLayer::new()
        .allow_methods([http::Method::POST])
        .allow_origin(Any)
        .allow_credentials(false)
        .allow_headers([http::header::CONTENT_TYPE, http::header::ACCEPT]);

    ApiRouter::new().route("/", post(event_handler)).layer(GovernorLayer::new(limiter)).layer(browser_cors)
}

const QUEUE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(serde::Deserialize, JsonSchema)]
struct EventRequest {
    entity_id: String,
    name: String,
    url: String,
    referrer: Option<String>,
    screen_width: Option<String>,
    orientation: Option<String>,
    properties: Option<RawEventProperties>,
    #[serde(default)]
    exit: bool,
}

pub(super) type RawEventProperties = std::collections::BTreeMap<String, serde_json::Value>;

const MAX_PROPERTIES: usize = 30;
const MAX_PROPERTY_VALUE_CHARS: usize = 255;

pub(super) struct ProcessEventRequest {
    pub(super) entity_id: String,
    pub(super) name: String,
    pub(super) url: Url,
    pub(super) referrer: Option<String>,
    pub(super) screen_width: Option<String>,
    pub(super) orientation: Option<String>,
    pub(super) properties: EventProperties,
    pub(super) created_at: DateTime<Utc>,
    pub(super) user_agent: Option<String>,
    pub(super) ip: Option<IpAddr>,
    pub(super) exit: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Utm {
    source: Option<String>,
    content: Option<String>,
    medium: Option<String>,
    campaign: Option<String>,
    term: Option<String>,
}

fn extract_query(url: &Url, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| url.query_pairs().find(|(name, _)| name == *key).map(|(_, value)| value.into_owned()))
        .filter(|value| !value.trim().is_empty() && value.len() <= 255)
}

fn extract_utm(url: &Url) -> Utm {
    Utm {
        campaign: extract_query(url, &["utm_campaign", "campaign"]),
        content: extract_query(url, &["utm_content", "content"]),
        medium: extract_query(url, &["utm_medium", "medium"]),
        source: extract_query(url, &["utm_source", "source", "ref", "referrer", "referer"]),
        term: extract_query(url, &["utm_term", "term"]),
    }
}

async fn event_handler(
    state: State<RouterState>,
    ClientIp(ip): ClientIp,
    geo_headers: GeoLocationHeaders,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
    event: Bytes,
) -> ApiResult<impl IntoApiResponse> {
    // we accept any content type, so we need to manually parse the body as JSON
    let event: EventRequest =
        serde_json::from_slice(&event).context("invalid json").http_err("invalid json", StatusCode::BAD_REQUEST)?;
    if event.entity_id.trim().is_empty() || event.entity_id.len() > 255 {
        http_bail!(StatusCode::BAD_REQUEST, "invalid entity_id")
    }
    let url = Url::from_str(&event.url).context("invalid url").http_err("invalid url", StatusCode::BAD_REQUEST)?;
    let properties = event_properties(event.properties)
        .context("invalid event")
        .http_err("invalid event", StatusCode::BAD_REQUEST)?;
    let app = state.app.clone();
    let request = ProcessEventRequest {
        entity_id: event.entity_id,
        name: event.name,
        url,
        referrer: event.referrer,
        screen_width: event.screen_width,
        orientation: event.orientation,
        properties,
        created_at: Utc::now(),
        user_agent: Some(user_agent.as_str().to_string()),
        ip,
        exit: event.exit,
    };
    validate_process_request(&request).context("invalid event").http_err("invalid event", StatusCode::BAD_REQUEST)?;

    // blocking a bit to give some slight backpressure to the caller
    let res = tokio::task::spawn_blocking(move || process_event(app, request, geo_headers, None))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
        .http_err("Failed to process event", StatusCode::INTERNAL_SERVER_ERROR)?;

    match res {
        Some((event, false)) => enqueue_events(&state, std::iter::once(event)).await?,
        Some((event, true)) => {
            let exit = EventExit {
                entity_id: event.entity_id,
                visitor_group_id: event.visitor_group_id,
                event: event.event,
                created_at: event.created_at,
                fqdn: event.fqdn,
                path: event.path,
            };
            state
                .exits
                .send_timeout(exit, QUEUE_TIMEOUT)
                .await
                .http_err("Event ingestion is unavailable", StatusCode::SERVICE_UNAVAILABLE)?;
        }
        // event was filtered out, do nothing
        None => {}
    };

    Ok(empty_response())
}

pub(super) async fn enqueue_events(state: &RouterState, events: impl ExactSizeIterator<Item = Event>) -> ApiResult<()> {
    let unavailable = || crate::web::webext::ApiError {
        message: "Event ingestion is unavailable".to_string(),
        status: StatusCode::SERVICE_UNAVAILABLE,
        retry_after: Some(1),
    };
    let permits = tokio::time::timeout(QUEUE_TIMEOUT, state.events.reserve_many(events.len()))
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
    for (permit, event) in permits.zip(events) {
        permit.send(event);
    }
    Ok(())
}

pub(super) fn process_event(
    app: Arc<Liwan>,
    event: ProcessEventRequest,
    geo_headers: GeoLocationHeaders,
    settings: Option<&ResolvedCollectionSettings>,
) -> Result<Option<(Event, bool)>> {
    let url = event.url;
    let is_exit = event.exit;
    let referrer = match process_referer(event.referrer.as_deref()) {
        Referrer::Fqdn(fqdn) => Some(fqdn),
        Referrer::Unknown(r) => r,
        Referrer::Spammer => return Ok(None),
        Referrer::Local => return Ok(None),
    };
    let referrer = referrer.map(|r| r.trim_start_matches("www.").to_string()); // remove www. prefix
    let referrer = referrer.filter(|r| r.trim().len() > 3); // ignore empty or short referrers

    if settings.is_none() && !app.entities.exists(&event.entity_id)? {
        return Ok(None);
    }

    let resolved_settings = settings.is_none().then(|| app.settings.resolved_for_entity(&event.entity_id));
    let settings = settings.or(resolved_settings.as_ref()).expect("settings are resolved above");
    let fqdn = url.host_str().unwrap_or_default().to_string();
    if !hostname_allowed(&fqdn, &settings.allowed_hostnames) {
        return Ok(None);
    }

    let client = if let Some(user_agent) = event.user_agent.as_deref() {
        if useragent::is_crawler_header(user_agent) {
            return Ok(None);
        }
        let client = useragent::parse(user_agent);
        if client.is_bot() {
            return Ok(None);
        }
        Some(client)
    } else {
        None
    };

    if is_exit
        && (event.name != "pageview"
            || !settings.track_sessions
            || settings.visitor_group_mode == VisitorGroupMode::RandomPerRequest
            || event.ip.is_none())
    {
        return Ok(None);
    }

    let visitor_group_id = if event.user_agent.is_none() {
        visitor_group_id_fallback()
    } else {
        resolve_visitor_group_id(
            settings,
            event.ip,
            event.user_agent.as_deref().unwrap_or_default(),
            &app.events.get_salt()?,
            &event.entity_id,
        )
    };

    let (country, city) = match settings.track_geo {
        GeoDetail::None => (None, None),
        GeoDetail::Country => (geo_headers.country, None),
        GeoDetail::City => (geo_headers.country, geo_headers.city),
    };

    #[cfg(feature = "geoip")]
    let (country, city) = match settings.track_geo {
        GeoDetail::None => (None, None),
        GeoDetail::Country => {
            let maxmind_country =
                event.ip.and_then(|ip| app.geoip.lookup(&ip).ok()).and_then(|lookup| lookup.country_code);
            (maxmind_country.or(country), None)
        }
        GeoDetail::City => {
            let lookup = event.ip.and_then(|ip| app.geoip.lookup(&ip).ok());
            (
                lookup.as_ref().and_then(|value| value.country_code.clone()).or(country),
                lookup.and_then(|value| value.city).or(city),
            )
        }
    };

    let utm = if settings.track_utm_params { extract_utm(&url) } else { Utm::default() };
    let path = url.path().to_string();
    let path = if path.len() > 1 && path.ends_with('/') { path.trim_end_matches('/').to_string() } else { path };

    let event = Event {
        visitor_group_id,
        referrer,
        country,
        city,
        mobile: client.as_ref().map(|client| client.is_mobile()),
        browser: client.as_ref().and_then(|client| client.ua_family.clone()),
        platform: client.and_then(|client| client.os_family),
        created_at: event.created_at,
        entity_id: event.entity_id,
        event: event.name,
        fqdn: fqdn.into(),
        path: path.into(),
        utm_campaign: utm.campaign,
        utm_content: utm.content,
        utm_medium: utm.medium,
        utm_source: utm.source,
        utm_term: utm.term,
        screen_width: event.screen_width,
        orientation: event.orientation,
        properties: event.properties,
        track_sessions: settings.track_sessions,
    };

    if settings.ingest_drop_rules.iter().any(|rule| ingest_drop_rule_matches(&event, rule)) {
        return Ok(None);
    }

    Ok(Some((event, is_exit)))
}

pub(super) fn validate_process_request(event: &ProcessEventRequest) -> Result<()> {
    if event.name.trim().is_empty() || event.name.len() > 255 {
        anyhow::bail!("name must be between 1 and 255 characters");
    }
    if event.url.as_str().len() > 2048 {
        anyhow::bail!("url cannot be longer than 2048 characters");
    }
    if event.referrer.as_deref().is_some_and(|value| value.len() > 256) {
        anyhow::bail!("referrer cannot be longer than 256 characters");
    }
    if event.user_agent.as_deref().is_some_and(|value| value.len() > 1024) {
        anyhow::bail!("userAgent cannot be longer than 1024 characters");
    }
    if event.screen_width.as_deref().is_some_and(|value| value.len() > 20) {
        anyhow::bail!("screenWidth cannot be longer than 20 characters");
    }
    if event.orientation.as_deref().is_some_and(|value| value.len() > 20) {
        anyhow::bail!("orientation cannot be longer than 20 characters");
    }
    Ok(())
}

/// Validate client properties and coerce their values to strings.
pub(super) fn event_properties(properties: Option<RawEventProperties>) -> Result<EventProperties> {
    let mut result = EventProperties::new();
    for (key, value) in properties.unwrap_or_default() {
        let value = match value {
            serde_json::Value::Null => continue,
            serde_json::Value::String(value) if value.is_empty() => continue,
            serde_json::Value::String(value) => value,
            serde_json::Value::Number(value) => value.to_string(),
            serde_json::Value::Bool(value) => value.to_string(),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                anyhow::bail!("property values must be strings, numbers, or booleans")
            }
        };
        let key = key.trim();
        crate::app::reports::validate_property_key(key)?;
        if value.chars().count() > MAX_PROPERTY_VALUE_CHARS {
            anyhow::bail!("property values cannot be longer than {MAX_PROPERTY_VALUE_CHARS} characters");
        }
        result.insert(key.to_string(), value);
    }
    if result.len() > MAX_PROPERTIES {
        anyhow::bail!("events cannot have more than {MAX_PROPERTIES} properties");
    }
    Ok(result)
}

fn ingest_drop_rule_matches(event: &Event, rule: &IngestDropRule) -> bool {
    !rule.filters.is_empty() && rule.filters.iter().all(|filter| ingest_filter_matches(event, filter))
}

fn ingest_filter_matches(event: &Event, filter: &IngestFilter) -> bool {
    if filter.dimension == "mobile" {
        return match filter.filter_type {
            FilterType::IsNull => event.mobile.is_none(),
            FilterType::IsTrue => event.mobile == Some(true),
            FilterType::IsFalse => event.mobile == Some(false),
            _ => false,
        };
    }

    let url;
    let value = match filter.dimension.as_str() {
        "event" => Some(event.event.as_str()),
        "url" => {
            url = format!("{}{}", event.fqdn.as_deref().unwrap_or_default(), event.path.as_deref().unwrap_or_default());
            Some(url.as_str())
        }
        "fqdn" => event.fqdn.as_deref(),
        "path" => event.path.as_deref(),
        "referrer" => event.referrer.as_deref(),
        "country" => event.country.as_deref(),
        "city" => event.city.as_deref(),
        "platform" => event.platform.as_deref(),
        "browser" => event.browser.as_deref(),
        "utm_source" => event.utm_source.as_deref(),
        "utm_medium" => event.utm_medium.as_deref(),
        "utm_campaign" => event.utm_campaign.as_deref(),
        "utm_content" => event.utm_content.as_deref(),
        "utm_term" => event.utm_term.as_deref(),
        "screen_width" => event.screen_width.as_deref(),
        "orientation" => event.orientation.as_deref(),
        _ => return false,
    };

    match filter.filter_type {
        FilterType::IsNull => value.is_none(),
        FilterType::Equal => {
            value.zip(filter.value.as_deref()).is_some_and(|(value, filter)| value.eq_ignore_ascii_case(filter))
        }
        FilterType::Contains => value
            .zip(filter.value.as_deref())
            .is_some_and(|(value, filter)| value.to_ascii_lowercase().contains(&filter.to_ascii_lowercase())),
        FilterType::StartsWith => value
            .zip(filter.value.as_deref())
            .is_some_and(|(value, filter)| value.to_ascii_lowercase().starts_with(&filter.to_ascii_lowercase())),
        FilterType::EndsWith => value
            .zip(filter.value.as_deref())
            .is_some_and(|(value, filter)| value.to_ascii_lowercase().ends_with(&filter.to_ascii_lowercase())),
        _ => false,
    }
}

fn resolve_visitor_group_id(
    settings: &ResolvedCollectionSettings,
    ip: Option<IpAddr>,
    user_agent: &str,
    daily_salt: &str,
    entity_id: &str,
) -> String {
    match (settings.visitor_group_mode, ip) {
        (VisitorGroupMode::RandomPerRequest, _) | (_, None) => visitor_group_id_fallback(),
        (VisitorGroupMode::Accurate, Some(ip)) => visitor_group_id(&ip, user_agent, daily_salt, entity_id),
        (mode, Some(ip)) => {
            let Some((ipv4_prefix, ipv6_prefix)) = mode.cidr_prefixes() else {
                return visitor_group_id_fallback();
            };
            visitor_group_id_cidr(&ip, ipv4_prefix, ipv6_prefix, daily_salt, entity_id)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn utm_alias_priority() {
        let url = Url::parse(
            "https://example.com/path/?utm_source=newsletter&source=ignored&campaign=spring&utm_medium=email&foo=bar&ref=backup",
        )
        .expect("valid url");

        let utm = extract_utm(&url);

        assert_eq!(utm.source.as_deref(), Some("newsletter"));
        assert_eq!(utm.medium.as_deref(), Some("email"));
        assert_eq!(utm.campaign.as_deref(), Some("spring"));
        assert_eq!(utm.content, None);
        assert_eq!(utm.term, None);
    }

    #[test]
    fn ingest_drop_rule_requires_all_filters_to_match() {
        let event = Event {
            event: "signup".to_string(),
            fqdn: Some("example.com".to_string()),
            path: Some("/pricing".to_string()),
            utm_source: Some("newsletter".to_string()),
            ..Default::default()
        };
        let filter = |dimension: &str, filter_type, value: Option<&str>| IngestFilter {
            dimension: dimension.to_string(),
            filter_type,
            value: value.map(str::to_string),
        };

        let matching_rule = IngestDropRule {
            filters: vec![
                filter("event", FilterType::Equal, Some("signup")),
                filter("path", FilterType::Equal, Some("/pricing")),
                filter("utm_source", FilterType::Equal, Some("newsletter")),
            ],
        };
        let non_matching_rule = IngestDropRule {
            filters: vec![
                filter("path", FilterType::Equal, Some("/pricing")),
                filter("utm_source", FilterType::Equal, Some("ads")),
            ],
        };

        assert!(ingest_drop_rule_matches(&event, &matching_rule));
        assert!(!ingest_drop_rule_matches(&event, &non_matching_rule));
        assert!(!ingest_drop_rule_matches(&event, &IngestDropRule { filters: Vec::new() }));
        // unknown dimensions never match, not even as null
        assert!(!ingest_filter_matches(&event, &filter("unknown", FilterType::IsNull, None)));
    }

    #[test]
    fn event_properties_coerce_and_validate() {
        let properties = |value: serde_json::Value| event_properties(Some(serde_json::from_value(value).unwrap()));

        let parsed = properties(serde_json::json!({
            " plan ": "pro",
            "count": 3,
            "ratio": 1.5,
            "beta": true,
            "unset": null,
            "empty": "",
        }))
        .unwrap();
        assert_eq!(
            parsed.into_iter().collect::<Vec<_>>(),
            [("beta", "true"), ("count", "3"), ("plan", "pro"), ("ratio", "1.5")]
                .map(|(key, value)| (key.to_string(), value.to_string()))
        );
        assert!(event_properties(None).unwrap().is_empty());

        assert!(properties(serde_json::json!({ "nested": { "a": 1 } })).is_err());
        assert!(properties(serde_json::json!({ "list": [1] })).is_err());
        assert!(properties(serde_json::json!({ "  ": "value" })).is_err());
        assert!(properties(serde_json::json!({ "k".repeat(65): "value" })).is_err());
        assert!(properties(serde_json::json!({ "key": "v".repeat(256) })).is_err());
        assert!(properties(serde_json::json!({ "k".repeat(64): "ü".repeat(255) })).is_ok());

        let many = (0..31).map(|i| (format!("key{i}"), serde_json::json!("value"))).collect::<RawEventProperties>();
        assert!(event_properties(Some(many.clone())).is_err());
        assert!(event_properties(Some(many.into_iter().take(30).collect())).is_ok());
    }
}
