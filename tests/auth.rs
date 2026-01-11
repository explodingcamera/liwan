use anyhow::Result;
use liwan::app::models::{self, UserRole};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_login() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.users.create("test", "test", UserRole::User, &[])?;

    // login
    let login = json!({ "username": "test", "password": "test" });
    let res = client.post("/api/dashboard/auth/login", login).await;

    res.assert_status_success();
    let cookies = common::cookies(&res);

    // user info
    let res = client
        .get_with_headers("/api/dashboard/auth/me", vec![("cookie".to_string(), common::cookie_header(&cookies))])
        .await;
    res.assert_status_success();
    let json: serde_json::Value = res.json();
    assert_eq!(json, json!({ "username": "test", "role": "user" }));

    // logout
    let res = client
        .post_with_headers(
            "/api/dashboard/auth/logout",
            json!({}),
            vec![("cookie".to_string(), common::cookie_header(&cookies))],
        )
        .await;
    res.assert_status_success();

    // test that the user is logged out
    let res = client
        .get_with_headers("/api/dashboard/auth/me", vec![("cookie".to_string(), common::cookie_header(&cookies))])
        .await;

    res.assert_status_unauthorized();
    Ok(())
}

#[tokio::test]
async fn test_setup() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    let token = app.onboarding.token().unwrap().expect("onboarding should exist");

    // Invalid token should return 401
    let setup = json!({ "token": "invalid_token", "username": "admin2", "password": "admin2" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_unauthorized();

    // Valid token should return 200
    let setup = json!({ "token": token, "username": "admin", "password": "admin" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_success();

    // Check that the user is created
    let login = json!({ "username": "admin", "password": "admin" });
    let res = client.post("/api/dashboard/auth/login", login).await;
    res.assert_status_success();

    // Check that the onboarding is cleared
    assert_eq!(app.onboarding.token().unwrap(), None, "onboarding should be cleared");
    let setup = json!({ "token": token, "username": "admin", "password": "admin" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_unauthorized();

    let setup = json!({ "token": token, "username": "admin2", "password": "admin2" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_unauthorized();

    Ok(())
}

#[tokio::test]
async fn expired_session() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.users.create("test", "test", UserRole::User, &[])?;

    // login
    let login = json!({ "username": "test", "password": "test" });
    let res = client.post("/api/dashboard/auth/login", login).await;
    res.assert_status_success();
    let cookies = common::cookies(&res);

    let session_id = cookies.iter().find(|cookie| cookie.name() == "liwan-session").unwrap().value().to_string();

    // user info
    let res = client
        .get_with_headers("/api/dashboard/auth/me", vec![("cookie".to_string(), common::cookie_header(&cookies))])
        .await;
    res.assert_status_success();
    let json: serde_json::Value = res.json();
    assert_eq!(json, json!({ "username": "test", "role": "user" }));

    // expire the session
    app.sessions.delete(&session_id)?;

    // test that the user is logged out
    let res = client
        .get_with_headers("/api/dashboard/auth/me", vec![("cookie".to_string(), common::cookie_header(&cookies))])
        .await;
    res.assert_status_unauthorized();

    Ok(())
}

#[tokio::test]
async fn private_projects() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.projects.create(
        &models::Project {
            display_name: "Private Project".to_string(),
            id: "private-project".to_string(),
            public: false,
            secret: None,
        },
        &[],
    )?;

    let res = client.get("/api/dashboard/projects").await;
    res.assert_json(&json!({"projects": []}));

    app.users.create("test", "test", UserRole::User, &[])?;
    app.users.create("test2", "test", UserRole::User, &["private-project"])?;

    let login1 = common::login(&client, "test", "test").await;
    let login2 = common::login(&client, "test2", "test").await;

    let res = client
        .get_with_headers("/api/dashboard/projects", vec![("cookie".to_string(), common::cookie_header(&login1))])
        .await;
    res.assert_json(&json!({"projects": []}));

    let res = client
        .get_with_headers("/api/dashboard/projects", vec![("cookie".to_string(), common::cookie_header(&login2))])
        .await;
    res.assert_json(&json!({"projects": [{"displayName": "Private Project", "id": "private-project", "public": false, "entities": []}]}));

    Ok(())
}
