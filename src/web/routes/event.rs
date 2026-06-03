use crate::app::models::{
    FilterType, GeoDetail, IngestDropRule, IngestFilter, ResolvedCollectionSettings, VisitorGroupMode,
};
use crate::app::{Liwan, models::Event};
use crate::utils::hash::{visitor_group_id, visitor_group_id_cidr, visitor_group_id_fallback};
use crate::utils::referrer::{Referrer, process_referer};
use crate::utils::useragent;
use crate::web::RouterState;
use crate::web::webext::{ApiResult, AxumErrExt, ClientIp, empty_response};

use aide::axum::routing::post;
use aide::axum::{ApiRouter, IntoApiResponse};
use anyhow::{Context, Result};
use axum::Json;
use axum::extract::State;
use axum_extra::TypedHeader;
use chrono::Utc;
use http::StatusCode;
use schemars::JsonSchema;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use url::Url;

pub fn router() -> ApiRouter<RouterState> {
    let limiter =
        GovernorConfigBuilder::default().per_second(2).burst_size(10).finish().expect("valid governor config");
    let governor_limiter = limiter.limiter().clone();

    tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_hours(1));
        loop {
            interval.tick().await;
            governor_limiter.retain_recent();
        }
    });

    ApiRouter::new().layer(GovernorLayer::new(limiter)).route("/event", post(event_handler))
}

#[derive(serde::Deserialize, JsonSchema)]
struct EventRequest {
    entity_id: String,
    name: String,
    url: String,
    referrer: Option<String>,
    screen_width: Option<String>,
    orientation: Option<String>,
}

impl EventRequest {
    fn validate(&self) -> Result<()> {
        if self.entity_id.trim().is_empty() {
            anyhow::bail!("entity_id cannot be empty");
        }
        if self.name.trim().is_empty() {
            anyhow::bail!("name cannot be empty");
        }

        if self.entity_id.len() > 255 {
            anyhow::bail!("entity_id cannot be longer than 255 characters");
        }

        if self.name.len() > 255 {
            anyhow::bail!("name cannot be longer than 255 characters");
        }

        if self.screen_width.as_deref().is_some_and(|w| w.len() > 20) {
            anyhow::bail!("screen_width cannot be longer than 20 characters");
        }

        if self.orientation.as_deref().is_some_and(|o| o.len() > 20) {
            anyhow::bail!("orientation cannot be longer than 20 characters");
        }

        if self.referrer.as_deref().is_some_and(|r| r.len() > 256) {
            anyhow::bail!("referrer cannot be longer than 256 characters");
        }

        if self.url.len() > 2048 {
            anyhow::bail!("url cannot be longer than 2048 characters");
        }

        Ok(())
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Utm {
    source: Option<String>,
    content: Option<String>,
    medium: Option<String>,
    campaign: Option<String>,
    term: Option<String>,
}

fn extract_query(url: &mut Url, keys: &[&str]) -> Option<String> {
    let value = keys
        .iter()
        .find_map(|key| url.query_pairs().find(|(name, _)| name == *key).map(|(_, value)| value.into_owned()));

    if let Some(value) = &value {
        let filtered = url
            .query_pairs()
            .filter(|(name, _)| !keys.contains(&name.as_ref()))
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();

        let mut pairs = url.query_pairs_mut();
        pairs.clear();
        drop(pairs);

        if !filtered.is_empty() {
            let mut pairs = url.query_pairs_mut();
            pairs.extend_pairs(filtered.iter().map(|(name, value)| (name.as_str(), value.as_str())));
        }

        if value.trim().is_empty() {
            return None;
        }

        if value.len() > 255 {
            return None;
        }
    }

    value
}

fn extract_utm(url: &mut Url) -> Utm {
    Utm {
        campaign: extract_query(url, &["utm_campaign", "campaign"]),
        content: extract_query(url, &["utm_content", "content"]),
        medium: extract_query(url, &["utm_medium", "medium"]),
        source: extract_query(url, &["utm_source", "source", "ref", "referrer", "referer"]),
        term: extract_query(url, &["utm_term", "term"]),
    }
}

static EXISTING_ENTITIES: LazyLock<quick_cache::sync::Cache<String, ()>> =
    LazyLock::new(|| quick_cache::sync::Cache::new(512));

async fn event_handler(
    state: State<RouterState>,
    ClientIp(ip): ClientIp,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
    Json(event): Json<EventRequest>,
) -> ApiResult<impl IntoApiResponse> {
    let url = Url::from_str(&event.url).context("invalid url").http_err("invalid url", StatusCode::BAD_REQUEST)?;
    let app = state.app.clone();
    let events = state.events.clone();
    event.validate().context("invalid event").http_err("invalid event", StatusCode::BAD_REQUEST)?;

    // run the event processing in the background
    let res = tokio::task::spawn_blocking(move || process_event(app, event, url, ip, user_agent))
        .await
        .http_status(StatusCode::INTERNAL_SERVER_ERROR)?;

    match res {
        Ok(Some(event)) => {
            if events.send_timeout(event, std::time::Duration::from_secs(2)).await.is_err() {
                tracing::warn!("Failed to send event, channel full");
            }
        }
        // event was filtered out, do nothing
        Ok(None) => {}
        Err(e) => tracing::warn!("Failed to process event: {:?}", e),
    };

    Ok(empty_response())
}

fn process_event(
    app: Arc<Liwan>,
    event: EventRequest,
    mut url: Url,
    ip: Option<IpAddr>,
    user_agent: headers::UserAgent,
) -> Result<Option<Event>> {
    let referrer = match process_referer(event.referrer.as_deref()) {
        Referrer::Fqdn(fqdn) => Some(fqdn),
        Referrer::Unknown(r) => r,
        Referrer::Spammer => return Ok(None),
        Referrer::Local => return Ok(None),
    };
    let referrer = referrer.map(|r| r.trim_start_matches("www.").to_string()); // remove www. prefix
    let referrer = referrer.filter(|r| r.trim().len() > 3); // ignore empty or short referrers

    if EXISTING_ENTITIES.get(&event.entity_id).is_none() {
        if !app.entities.exists(&event.entity_id).unwrap_or(false) {
            return Ok(None);
        }
        EXISTING_ENTITIES.insert(event.entity_id.clone(), ());
    }

    let settings = app.settings.resolved_for_entity(&event.entity_id);

    // we delay the user agent parsing as much as possible since it's by far the most expensive operation
    let client = useragent::parse(user_agent.as_str());
    if client.is_bot() {
        return Ok(None);
    }

    let visitor_group_id =
        resolve_visitor_group_id(&settings, ip, user_agent.as_str(), &app.events.get_salt()?, &event.entity_id);

    #[cfg(feature = "geoip")]
    let (country, city) = match settings.track_geo {
        GeoDetail::None => (None, None),
        GeoDetail::Country => ip
            .and_then(|ip| app.geoip.lookup(&ip).ok())
            .map(|lookup| (lookup.country_code, None))
            .unwrap_or((None, None)),
        GeoDetail::City => ip
            .and_then(|ip| app.geoip.lookup(&ip).ok())
            .map(|lookup| (lookup.country_code, lookup.city))
            .unwrap_or((None, None)),
    };

    #[cfg(not(feature = "geoip"))]
    let (country, city) = (None, None);

    let utm = if settings.track_utm_params { extract_utm(&mut url) } else { Utm::default() };
    url.set_query(None);
    let path = url.path().to_string();
    let path = if path.len() > 1 && path.ends_with('/') { path.trim_end_matches('/').to_string() } else { path };
    let fqdn = url.host_str().unwrap_or_default().to_string();

    let event = Event {
        visitor_group_id,
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
        utm_campaign: utm.campaign,
        utm_content: utm.content,
        utm_medium: utm.medium,
        utm_source: utm.source,
        utm_term: utm.term,
        screen_width: event.screen_width,
        orientation: event.orientation,
        track_sessions: settings.track_sessions,
    };

    if settings.ingest_drop_rules.iter().any(|rule| ingest_drop_rule_matches(&event, rule)) {
        return Ok(None);
    }

    Ok(Some(event))
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
    fn extract_utm_clears_all_query_params() {
        let mut url = Url::parse(
            "https://example.com/path/?utm_source=newsletter&source=ignored&campaign=spring&utm_medium=email&foo=bar&ref=backup",
        )
        .expect("valid url");

        let utm = extract_utm(&mut url);
        url.set_query(None);

        assert_eq!(utm.source.as_deref(), Some("newsletter"));
        assert_eq!(utm.medium.as_deref(), Some("email"));
        assert_eq!(utm.campaign.as_deref(), Some("spring"));
        assert_eq!(utm.content, None);
        assert_eq!(utm.term, None);
        assert_eq!(url.as_str(), "https://example.com/path/");
    }

    #[test]
    fn unknown_ingest_filter_dimension_does_not_match_null() {
        let event = Event {
            entity_id: "entity".to_string(),
            visitor_group_id: "visitor".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: None,
            path: None,
            referrer: None,
            platform: None,
            browser: None,
            mobile: None,
            country: None,
            city: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            utm_content: None,
            utm_term: None,
            screen_width: None,
            orientation: None,
            track_sessions: true,
        };

        assert!(!ingest_filter_matches(
            &event,
            &IngestFilter { dimension: "unknown".to_string(), filter_type: FilterType::IsNull, value: None },
        ));
    }

    #[test]
    fn ingest_drop_rule_requires_all_filters_to_match() {
        let event = Event {
            entity_id: "entity".to_string(),
            visitor_group_id: "visitor".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: Some("example.com".to_string()),
            path: Some("/pricing".to_string()),
            referrer: None,
            platform: None,
            browser: None,
            mobile: None,
            country: None,
            city: None,
            utm_source: Some("newsletter".to_string()),
            utm_medium: None,
            utm_campaign: None,
            utm_content: None,
            utm_term: None,
            screen_width: None,
            orientation: None,
            track_sessions: true,
        };

        let matching_rule = IngestDropRule {
            filters: vec![
                IngestFilter {
                    dimension: "path".to_string(),
                    filter_type: FilterType::Equal,
                    value: Some("/pricing".to_string()),
                },
                IngestFilter {
                    dimension: "utm_source".to_string(),
                    filter_type: FilterType::Equal,
                    value: Some("newsletter".to_string()),
                },
            ],
        };
        let non_matching_rule = IngestDropRule {
            filters: vec![
                IngestFilter {
                    dimension: "path".to_string(),
                    filter_type: FilterType::Equal,
                    value: Some("/pricing".to_string()),
                },
                IngestFilter {
                    dimension: "utm_source".to_string(),
                    filter_type: FilterType::Equal,
                    value: Some("ads".to_string()),
                },
            ],
        };

        assert!(ingest_drop_rule_matches(&event, &matching_rule));
        assert!(!ingest_drop_rule_matches(&event, &non_matching_rule));
    }

    #[test]
    fn empty_ingest_drop_rule_does_not_match() {
        let event = Event {
            entity_id: "entity".to_string(),
            visitor_group_id: "visitor".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: None,
            path: None,
            referrer: None,
            platform: None,
            browser: None,
            mobile: None,
            country: None,
            city: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            utm_content: None,
            utm_term: None,
            screen_width: None,
            orientation: None,
            track_sessions: true,
        };

        assert!(!ingest_drop_rule_matches(&event, &IngestDropRule { filters: Vec::new() }));
    }
}
