use std::{net::IpAddr, str::FromStr};

use aide::UseApi;
use aide::axum::{ApiRouter, IntoApiResponse, routing::post};
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::{DefaultBodyLimit, State},
};
use chrono::{DateTime, Utc};
use http::StatusCode;
use schemars::JsonSchema;
use url::Url;

use super::event::{ProcessEventRequest, enqueue_events, process_event, validate_process_request};
use crate::app::models::ApiPermission;
use crate::web::{
    RouterState,
    webext::{ApiResult, AuthenticatedApiKey, AxumErrExt, GeoLocationHeaders, http_bail},
};

const MAX_BATCH_EVENTS: usize = 10_000;
const MAX_BATCH_BODY_BYTES: usize = 10 * 1024 * 1024;
const MAX_FUTURE_SECONDS: i64 = 5 * 60;

pub fn router() -> ApiRouter<RouterState> {
    ApiRouter::new().api_route("/events", post(batch_event_handler)).layer(DefaultBodyLimit::max(MAX_BATCH_BODY_BYTES))
}

#[derive(serde::Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct BatchRequest {
    entity_id: String,
    events: Vec<BatchEventRequest>,
}

#[derive(serde::Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct BatchEventRequest {
    name: String,
    url: String,
    referrer: Option<String>,
    created_at: Option<String>,
    user_agent: Option<String>,
    ip: Option<String>,
    screen_width: Option<String>,
    orientation: Option<String>,
}

#[derive(serde::Serialize, JsonSchema)]
struct BatchResponse {
    accepted: usize,
    filtered: usize,
}

async fn batch_event_handler(
    state: State<RouterState>,
    authentication: AuthenticatedApiKey,
    Json(request): Json<BatchRequest>,
) -> ApiResult<UseApi<impl IntoApiResponse, Json<BatchResponse>>> {
    let AuthenticatedApiKey { access } = authentication;

    if request.entity_id.trim().is_empty() || request.entity_id.len() > 255 {
        http_bail!(StatusCode::BAD_REQUEST, "invalid entityId")
    }
    if !access.has_permission(ApiPermission::EventsBatch) || !access.can_access_entity(&request.entity_id) {
        http_bail!(StatusCode::FORBIDDEN, "API key cannot access this entity")
    }
    if request.events.is_empty() {
        http_bail!(StatusCode::BAD_REQUEST, "Batch must contain at least one event")
    }
    if request.events.len() > MAX_BATCH_EVENTS {
        http_bail!(StatusCode::PAYLOAD_TOO_LARGE, "Batch exceeds 10,000 events")
    }

    let requests = request
        .events
        .into_iter()
        .enumerate()
        .map(|(index, event)| {
            prepare_batch_event(&request.entity_id, event).with_context(|| format!("invalid event at index {index}"))
        })
        .collect::<Result<Vec<_>>>()
        .map_err(|error| crate::web::webext::ApiError {
            message: error.to_string(),
            status: StatusCode::BAD_REQUEST,
            retry_after: None,
        })?;

    let entity_id = request.entity_id;
    let processing_entity_id = entity_id.clone();
    let app = state.app.clone();
    let processed = tokio::task::spawn_blocking(move || {
        let settings = app.settings.resolved_for_entity(&processing_entity_id);
        requests
            .into_iter()
            .map(|request| {
                process_event(app.clone(), request, GeoLocationHeaders::default(), Some(&settings))
                    .map(|event| event.map(|(event, _)| event))
            })
            .collect::<Result<Vec<_>>>()
    })
    .await
    .http_status(StatusCode::INTERNAL_SERVER_ERROR)?
    .http_err("Failed to process event batch", StatusCode::INTERNAL_SERVER_ERROR)?;

    let filtered = processed.iter().filter(|event| event.is_none()).count();
    let events = processed.into_iter().flatten().collect::<Vec<_>>();
    let accepted = events.len();
    if !events.is_empty() {
        enqueue_events(&state, events.into_iter()).await?;
    }
    tracing::debug!(
        key_id = access.id,
        entity_id,
        batch_size = accepted + filtered,
        accepted,
        filtered,
        "Accepted event batch"
    );
    Ok((StatusCode::ACCEPTED, Json(BatchResponse { accepted, filtered })).into())
}

fn prepare_batch_event(entity_id: &str, event: BatchEventRequest) -> Result<ProcessEventRequest> {
    let ip = event.ip.as_deref().map(IpAddr::from_str).transpose().context("invalid IP address")?;
    let url = Url::from_str(&event.url).context("invalid URL")?;
    let created_at = event
        .created_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .context("invalid timestamp")?
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    if created_at > Utc::now() + chrono::Duration::seconds(MAX_FUTURE_SECONDS) {
        anyhow::bail!("timestamp is too far in the future");
    }
    let request = ProcessEventRequest {
        entity_id: entity_id.to_string(),
        name: event.name,
        url,
        referrer: event.referrer,
        screen_width: event.screen_width,
        orientation: event.orientation,
        created_at,
        user_agent: event.user_agent,
        ip,
        exit: false,
    };
    validate_process_request(&request)?;
    Ok(request)
}
