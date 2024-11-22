mod common;
use eyre::Result;
use liwan::app::models::Entity;
use poem::http::{header, status::StatusCode};
use serde_json::json;

#[tokio::test]
async fn test_event() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let router = common::router(app.clone(), tx);
    let client = poem::test::TestClient::new(router);
    app.entities.create(&Entity { display_name: "Entity 1".to_string(), id: "entity-1".to_string() }, &[])?;

    let event = &json!({
        "entity_id": "entity-1",
        "name": "pageview",
        "url": "https://example.com/"
    });

    // Require User-Agent
    let res = client.post("/api/event").body_json(event).send().await;
    res.assert_status(StatusCode::BAD_REQUEST);

    // Create event
    let res = client.post("/api/event").header(header::USER_AGENT, "test").body_json(event).send().await;
    res.assert_status_is_ok();

    Ok(())
}
