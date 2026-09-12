mod common;
use anyhow::Result;
use liwan::app::models::Entity;
use liwan::config::Config;
use liwan::utils::ip_headers::{ClientIpHeaderSource, TrustedProxy};
use serde_json::json;

#[tokio::test]
async fn test_event() -> Result<()> {
    let app = common::app();
    let (tx, mut rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.seed_database(0)?;

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

    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.events.recv())
        .await
        .expect("event should be received")
        .expect("event channel should not be closed");

    app.events.append(std::iter::once(event))?;

    let start = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let end = chrono::Utc::now().to_rfc3339();

    let res = client
        .post(
            "/api/dashboard/project/public-project/dimension",
            json!({
                "dimension": "url",
                "filters": [],
                "metric": "views",
                "range": { "start": start, "end": end }
            }),
        )
        .await;
    res.assert_status_success();

    let body: serde_json::Value = res.json();
    let rows = body["data"].as_array().expect("data should be an array");
    let row = rows.iter().find(|r| r["dimensionValue"].as_str() == Some("example.com/")).expect("url row should exist");
    assert_eq!(row["value"].as_f64(), Some(1.0));

    Ok(())
}

#[tokio::test]
async fn event_allows_cross_origin_requests() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.seed_database(0)?;

    let event = json!({
        "entity_id": "entity-1",
        "name": "pageview",
        "url": "https://example.com/"
    });

    let res = client
        .post_with_headers(
            "/api/event",
            event,
            vec![
                ("origin".to_string(), "https://site.example".to_string()),
                ("user-agent".to_string(), "test".to_string()),
            ],
        )
        .await;
    res.assert_status_success();

    Ok(())
}

#[tokio::test]
async fn deleted_entity_does_not_accept_events() -> Result<()> {
    let app = common::app();
    let (tx, mut rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.entities
        .create(&Entity { id: "entity-to-delete".to_string(), display_name: "Entity to delete".to_string() }, &[])?;

    let event = json!({
        "entity_id": "entity-to-delete",
        "name": "pageview",
        "url": "https://example.com/"
    });
    let headers = vec![("user-agent".to_string(), "test".to_string())];

    client.post_with_headers("/api/event", event.clone(), headers.clone()).await.assert_status_success();
    rx.events.recv().await.expect("event should be received");

    app.entities.delete("entity-to-delete")?;
    client.post_with_headers("/api/event", event, headers).await.assert_status_success();
    assert!(rx.events.try_recv().is_err(), "deleted entity should not produce an event");

    Ok(())
}

#[tokio::test]
async fn exit_payload_uses_the_exit_queue() -> Result<()> {
    let mut config = Config::default();
    config.trusted_headers = vec![ClientIpHeaderSource::Header("x-client-ip".to_string())].into();
    config.trusted_proxies = vec![TrustedProxy::Ip("127.0.0.1".parse()?)].into();
    let app = liwan::app::Liwan::new_memory(config)?;
    let (queues, mut receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.seed_database(0)?;

    let event = json!({
        "entity_id": "entity-1",
        "name": "pageview",
        "url": "https://example.com/",
        "exit": true
    });
    client
        .post_with_headers(
            "/api/event",
            event,
            vec![("user-agent".to_string(), "test".to_string()), ("x-client-ip".to_string(), "8.8.8.8".to_string())],
        )
        .await
        .assert_status_success();

    let exit = tokio::time::timeout(std::time::Duration::from_secs(1), receivers.exits.recv())
        .await
        .expect("exit should be received")
        .expect("exit channel should not be closed");
    assert_eq!(exit.event, "pageview");
    assert_eq!(exit.fqdn.as_deref(), Some("example.com"));
    assert!(receivers.events.try_recv().is_err(), "exit payload should not insert an event");

    Ok(())
}
