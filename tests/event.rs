mod common;
use anyhow::Result;
use serde_json::json;

#[tokio::test]
async fn test_event() -> Result<()> {
    let app = common::app();
    let (tx, rx) = common::events();
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

    let event = rx.recv_timeout(std::time::Duration::from_secs(1)).expect("event should be received");
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
