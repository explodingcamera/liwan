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
async fn test_event_screen_size_accepted() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.entities.create(&Entity { display_name: "Entity 1".to_string(), id: "entity-1".to_string() }, &[])?;

    let user_agent = vec![("user-agent".to_string(), "Mozilla/5.0 (Linux x86_64)".to_string())];
    let event_desktop_screen_size = json!({
        "entity_id": "entity-1",
        "name": "pageview",
        "url": "https://example.com/",
        "screen_width": 1920,
        "screen_height": 1080
    });

    let res = client.post_with_headers("/api/event", event_desktop_screen_size.clone(), user_agent.clone()).await;
    res.assert_status_success();

    let event_mobile_screen_size = json!({
        "entity_id": "entity-1",
        "name": "pageview",
        "url": "https://example.com/",
        "screen_width": 390,
        "screen_height": 844
    });

    let res = client.post_with_headers("/api/event", event_mobile_screen_size.clone(), user_agent.clone()).await;
    res.assert_status_success();

    Ok(())
}

#[tokio::test]
async fn test_event_screen_size_read_write() -> Result<()> {
    use chrono::Utc;
    use liwan::app::models::Event;

    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.seed_database(0)?;

    let events_to_insert = vec![
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-desktop".to_string(),
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
            screen_width: Some(1920),
            screen_height: Some(1080),
        },
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-mobile".to_string(),
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
            screen_width: Some(390),
            screen_height: Some(844),
        },
        Event {
            entity_id: "entity-1".to_string(),
            visitor_id: "visitor-old".to_string(),
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
            screen_width: None,
            screen_height: None,
        },
    ];
    app.events.append(events_to_insert.into_iter())?;

    let start = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let end = Utc::now().to_rfc3339();

    let query = json!({
        "dimension": "screen_resolution",
        "filters": [],
        "metric": "views",
        "range": { "start": start, "end": end }
    });

    let res = client.post("/api/dashboard/project/public-project/dimension", query.clone()).await;
    res.assert_status_success();

    let body: serde_json::Value = res.json();
    let rows = body["data"].as_array().expect("expected data array");
    let values: Vec<&str> = rows.iter().filter_map(|r| r["dimensionValue"].as_str()).collect();

    assert!(values.contains(&"1920x1080"), "expected 1920x1080 in results, got: {values:?}");
    assert!(values.contains(&"390x844"), "expected 390x844 in results, got: {values:?}");
    assert!(values.contains(&"Unknown"), "expected Unknown for NULL screen data, got: {values:?}");

    Ok(())
}
