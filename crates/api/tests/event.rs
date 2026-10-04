use std::time::UNIX_EPOCH;

use liwan_api::{IntoTimestamp, RequestMetadata};

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

    let request = http::Request::builder().uri("https://public.example/docs?section=api").body(()).unwrap();
    let (parts, _) = request.into_parts();
    let metadata = RequestMetadata::from_parts(&parts, ip);
    assert_eq!(metadata.url, "https://public.example/docs?section=api");
}

#[test]
fn formats_system_time_as_rfc3339() {
    assert_eq!(UNIX_EPOCH.into_timestamp(), "1970-01-01T00:00:00Z");
    assert_eq!((UNIX_EPOCH - std::time::Duration::from_secs(1)).into_timestamp(), "1969-12-31T23:59:59Z");
    assert_eq!((UNIX_EPOCH + std::time::Duration::from_secs(1_704_164_645)).into_timestamp(), "2024-01-02T03:04:05Z");
}
