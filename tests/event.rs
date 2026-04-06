mod common;
use anyhow::Result;
use liwan::app::models::Entity;
use serde_json::json;

#[tokio::test]
async fn test_event() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.entities.create(&Entity { display_name: "Entity 1".to_string(), id: "entity-1".to_string() }, &[])?;

    let event = json!({
        "entity_id": "entity-1",
        "name": "pageview",
        "url": "https://example.com/"
    });

    // Require User-Agent
    let res = client.post("/api/event", event.clone()).await;
    res.assert_status_bad_request();

    // Create event
    let res = client.post_with_headers("/api/event", event, vec![("user-agent".to_string(), "test".to_string())]).await;
    res.assert_status_success();

    Ok(())
}

#[tokio::test]
async fn test_event_screen_size() -> Result<()> {
    let app = common::app();
    let (tx, rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.entities.create(&Entity { display_name: "Entity 1".to_string(), id: "entity-1".to_string() }, &[])?;

    let ua = vec![("user-agent".to_string(), "Mozilla/5.0 (test)".to_string())];

    // Mobile: 375px
    let res = client
        .post_with_headers(
            "/api/event",
            json!({
                "entity_id": "entity-1", "name": "pageview",
                "url": "https://example.com/", "screen_width": 375
            }),
            ua.clone(),
        )
        .await;
    res.assert_status_success();
    let event = rx.recv().unwrap();
    assert_eq!(event.screen_size.as_deref(), Some("mobile"));

    // Tablet: 810px
    let res = client
        .post_with_headers(
            "/api/event",
            json!({
                "entity_id": "entity-1", "name": "pageview",
                "url": "https://example.com/", "screen_width": 810
            }),
            ua.clone(),
        )
        .await;
    res.assert_status_success();
    let event = rx.recv().unwrap();
    assert_eq!(event.screen_size.as_deref(), Some("tablet"));

    // Desktop: 1920px
    let res = client
        .post_with_headers(
            "/api/event",
            json!({
                "entity_id": "entity-1", "name": "pageview",
                "url": "https://example.com/", "screen_width": 1920
            }),
            ua.clone(),
        )
        .await;
    res.assert_status_success();
    let event = rx.recv().unwrap();
    assert_eq!(event.screen_size.as_deref(), Some("desktop"));

    // Ultrawide: 3840px
    let res = client
        .post_with_headers(
            "/api/event",
            json!({
                "entity_id": "entity-1", "name": "pageview",
                "url": "https://example.com/", "screen_width": 3840
            }),
            ua.clone(),
        )
        .await;
    res.assert_status_success();
    let event = rx.recv().unwrap();
    assert_eq!(event.screen_size.as_deref(), Some("ultrawide"));

    // Wuithout screen_width
    let res = client
        .post_with_headers(
            "/api/event",
            json!({
                "entity_id": "entity-1", "name": "pageview",
                "url": "https://example.com/"
            }),
            ua.clone(),
        )
        .await;
    res.assert_status_success();
    let event = rx.recv().unwrap();
    assert_eq!(event.screen_size, None);

    Ok(())
}

#[tokio::test]
async fn test_screen_size_dimension_api() -> Result<()> {
    use chrono::Utc;
    use liwan::app::models::Event;

    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.seed_database(0)?;

    let events_to_insert = vec![
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-1".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: Some("example.com".to_string()),
            path: Some("/".to_string()),
            referrer: None,
            platform: None,
            browser: None,
            mobile: Some(false),
            country: None,
            city: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            utm_content: None,
            utm_term: None,
            screen_size: Some("mobile".to_string()),
        },
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-2".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: Some("example.com".to_string()),
            path: Some("/".to_string()),
            referrer: None,
            platform: None,
            browser: None,
            mobile: Some(true),
            country: None,
            city: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            utm_content: None,
            utm_term: None,
            screen_size: Some("mobile".to_string()),
        },
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-2".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: Some("example.com".to_string()),
            path: Some("/".to_string()),
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
            screen_size: Some("tablet".to_string()),
        },
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-3".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: Some("example.com".to_string()),
            path: Some("/".to_string()),
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
            screen_size: Some("desktop".to_string()),
        },
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-4".to_string(),
            event: "pageview".to_string(),
            created_at: Utc::now(),
            fqdn: Some("example.com".to_string()),
            path: Some("/".to_string()),
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
            screen_size: Some("ultrawide".to_string()),
        },
    ];
    app.events.append(events_to_insert.into_iter())?;

    let start = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let end = Utc::now().to_rfc3339();

    let res = client
        .post(
            "/api/dashboard/project/public-project/dimension",
            json!({
                "dimension": "screen_size",
                "filters": [],
                "metric": "views",
                "range": { "start": start, "end": end }
            }),
        )
        .await;
    res.assert_status_success();

    let body: serde_json::Value = res.json();
    let rows = body["data"].as_array().expect("data should be an array");

    let find = |bucket: &str| rows.iter().find(|r| r["dimensionValue"].as_str() == Some(bucket));

    let mobile_row = find("mobile").expect("mobile bucket should be present");
    assert_eq!(mobile_row["displayName"].as_str(), Some("Mobile"));
    assert_eq!(mobile_row["value"].as_f64(), Some(2.0));

    let tablet_row = find("tablet").expect("tablet bucket should be present");
    assert_eq!(tablet_row["displayName"].as_str(), Some("Tablet"));
    assert_eq!(tablet_row["value"].as_f64(), Some(1.0));

    let desktop_row = find("desktop").expect("desktop bucket should be present");
    assert_eq!(desktop_row["displayName"].as_str(), Some("Desktop"));
    assert_eq!(desktop_row["value"].as_f64(), Some(1.0));

    let ultrawide_row = find("ultrawide").expect("ultrawide bucket should be present");
    assert_eq!(ultrawide_row["displayName"].as_str(), Some("Ultrawide"));
    assert_eq!(ultrawide_row["value"].as_f64(), Some(1.0));

    Ok(())
}
