use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use http::{HeaderMap, Uri, request::Parts};
use serde::Serialize;

/// Metadata captured from an HTTP request.
#[derive(Debug, Clone, Default)]
pub struct RequestMetadata {
    pub url: String,
    pub referrer: Option<String>,
    pub ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

impl RequestMetadata {
    /// Builds metadata from framework-neutral HTTP request parts and a trusted, resolved client IP.
    pub fn from_parts(parts: &Parts, client_ip: Option<IpAddr>) -> Self {
        Self::from_headers(&parts.uri, &parts.headers, client_ip, None)
    }

    pub(crate) fn from_headers(
        uri: &Uri,
        headers: &HeaderMap,
        client_ip: Option<IpAddr>,
        origin: Option<&str>,
    ) -> Self {
        let url = if uri.scheme().is_some() && uri.authority().is_some() {
            uri.to_string()
        } else if let Some(origin) = origin {
            format!("{origin}{uri}")
        } else {
            let host = headers.get(http::header::HOST).and_then(|value| value.to_str().ok()).unwrap_or("localhost");
            format!("http://{host}{uri}")
        };
        Self {
            url,
            referrer: headers.get(http::header::REFERER).and_then(|value| value.to_str().ok()).map(str::to_string),
            ip: client_ip,
            user_agent: headers.get(http::header::USER_AGENT).and_then(|value| value.to_str().ok()).map(str::to_string),
        }
    }
}

/// An analytics event sent to Liwan.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) referrer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ip: Option<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) screen_width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) orientation: Option<String>,
}

impl Event {
    /// Creates an event and records its observation time immediately.
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            created_at: timestamp_now(),
            referrer: None,
            ip: None,
            user_agent: None,
            screen_width: None,
            orientation: None,
        }
    }

    /// Creates a pageview event.
    pub fn pageview(url: impl Into<String>) -> Self {
        Self::new("pageview", url)
    }

    /// Applies request metadata to this event.
    pub fn with_request(mut self, metadata: RequestMetadata) -> Self {
        self.url = metadata.url;
        self.referrer = metadata.referrer;
        self.ip = metadata.ip;
        self.user_agent = metadata.user_agent;
        self
    }

    /// Overrides when the event was observed.
    pub fn created_at(mut self, created_at: impl IntoTimestamp) -> Self {
        self.created_at = created_at.into_timestamp();
        self
    }

    /// Adds a referrer.
    pub fn referrer(mut self, referrer: impl Into<String>) -> Self {
        self.referrer = Some(referrer.into());
        self
    }
}

/// Converts supported timestamp types to RFC 3339.
pub trait IntoTimestamp {
    fn into_timestamp(self) -> String;
}

#[cfg(feature = "jiff")]
impl IntoTimestamp for jiff::Timestamp {
    fn into_timestamp(self) -> String {
        self.to_string()
    }
}

impl IntoTimestamp for SystemTime {
    fn into_timestamp(self) -> String {
        format_system_time(self)
    }
}

#[cfg(feature = "chrono")]
impl<Tz: chrono::TimeZone> IntoTimestamp for chrono::DateTime<Tz>
where
    Tz::Offset: std::fmt::Display,
{
    fn into_timestamp(self) -> String {
        self.to_rfc3339()
    }
}

fn timestamp_now() -> String {
    #[cfg(feature = "jiff")]
    return jiff::Timestamp::now().to_string();
    #[cfg(all(not(feature = "jiff"), feature = "chrono"))]
    return chrono::Utc::now().to_rfc3339();
    #[cfg(not(any(feature = "jiff", feature = "chrono")))]
    return format_system_time(SystemTime::now());
}

fn format_system_time(time: SystemTime) -> String {
    let seconds = match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as i64,
        Err(error) => {
            let duration = error.duration();
            -(duration.as_secs() as i64) - i64::from(duration.subsec_nanos() > 0)
        }
    };
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    // Convert days since the Unix epoch to a Gregorian calendar date.
    let shifted_days = days + 719_468;
    let era = shifted_days.div_euclid(146_097);
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = month_index + if month_index < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_request_metadata() {
        let request = http::Request::builder()
            .uri("/docs?section=api")
            .header(http::header::HOST, "example.com")
            .header(http::header::REFERER, "https://example.com/")
            .header(http::header::USER_AGENT, "test-agent")
            .body(())
            .unwrap();
        let (parts, _) = request.into_parts();
        let ip = Some("192.0.2.1".parse().unwrap());

        let metadata = RequestMetadata::from_parts(&parts, ip);
        assert_eq!(metadata.url, "http://example.com/docs?section=api");
        assert_eq!(metadata.referrer.as_deref(), Some("https://example.com/"));
        assert_eq!(metadata.user_agent.as_deref(), Some("test-agent"));
        assert_eq!(metadata.ip, ip);

        let metadata = RequestMetadata::from_headers(&parts.uri, &parts.headers, ip, Some("https://public.example"));
        assert_eq!(metadata.url, "https://public.example/docs?section=api");
    }

    #[test]
    fn formats_system_time_as_rfc3339() {
        assert_eq!(format_system_time(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(format_system_time(UNIX_EPOCH - std::time::Duration::from_secs(1)), "1969-12-31T23:59:59Z");
        assert_eq!(
            format_system_time(UNIX_EPOCH + std::time::Duration::from_secs(1_704_164_645)),
            "2024-01-02T03:04:05Z"
        );
    }
}
