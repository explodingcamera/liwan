use anyhow::Result;
use liwan::app::models::{self, UserRole};
use poem::http::{header, status::StatusCode};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_login() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let router = common::router(app.clone(), tx);
    let client = poem::test::TestClient::new(router);

    app.users.create("test", "test", UserRole::User, &[])?;

    // login
    let login = &json!({ "username": "test", "password": "test" });
    let res = client.post("/api/dashboard/auth/login").body_json(login).send().await;

    res.assert_status_is_ok();
    let cookies = common::cookies(&res);

    // user info
    let res = client.get("/api/dashboard/auth/me").header(header::COOKIE, common::cookie_header(&cookies)).send().await;
    res.assert_status_is_ok();
    res.assert_json(json!({ "username": "test", "role": "user" })).await;

    // logout
    let res =
        client.post("/api/dashboard/auth/logout").header(header::COOKIE, common::cookie_header(&cookies)).send().await;
    res.assert_status_is_ok();

    // test that the user is logged out
    let res = client.get("/api/dashboard/auth/me").header(header::COOKIE, common::cookie_header(&cookies)).send().await;

    res.assert_status(StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn test_setup() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let router = common::router(app.clone(), tx);
    let client = poem::test::TestClient::new(router);

    let token = app.onboarding.token().unwrap().expect("onboarding should exist");

    // Invalid token should return 401
    let setup = &json!({ "token": "invalid_token", "username": "admin2", "password": "admin2" });
    let res = client.post("/api/dashboard/auth/setup").body_json(setup).send().await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    // Valid token should return 200
    let setup = &json!({ "token": token, "username": "admin", "password": "admin" });
    let res = client.post("/api/dashboard/auth/setup").body_json(setup).send().await;
    res.assert_status_is_ok();

    // Check that the user is created
    let login = &json!({ "username": "admin", "password": "admin" });
    let res = client.post("/api/dashboard/auth/login").body_json(login).send().await;
    res.assert_status_is_ok();

    // Check that the onboarding is cleared
    assert_eq!(app.onboarding.token().unwrap(), None, "onboarding should be cleared");
    let setup = &json!({ "token": token, "username": "admin", "password": "admin" });
    let res = client.post("/api/dashboard/auth/setup").body_json(setup).send().await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    let setup = &json!({ "token": token, "username": "admin2", "password": "admin2" });
    let res = client.post("/api/dashboard/auth/setup").body_json(setup).send().await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn expired_session() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let router = common::router(app.clone(), tx);
    let client = poem::test::TestClient::new(router);

    app.users.create("test", "test", UserRole::User, &[])?;

    // login
    let login = &json!({ "username": "test", "password": "test" });
    let res = client.post("/api/dashboard/auth/login").body_json(login).send().await;
    res.assert_status_is_ok();
    let cookies = common::cookies(&res);

    let session_id = cookies.iter().find(|cookie| cookie.name() == "liwan-session").unwrap().value().to_string();

    // user info
    let res = client.get("/api/dashboard/auth/me").header(header::COOKIE, common::cookie_header(&cookies)).send().await;
    res.assert_status_is_ok();
    res.assert_json(json!({ "username": "test", "role": "user" })).await;

    // expire the session
    app.sessions.delete(&session_id)?;

    // test that the user is logged out
    let res = client.get("/api/dashboard/auth/me").header(header::COOKIE, common::cookie_header(&cookies)).send().await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn private_projects() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let router = common::router(app.clone(), tx);
    let client = poem::test::TestClient::new(router);

    app.projects.create(
        &models::Project {
            display_name: "Private Project".to_string(),
            id: "private-project".to_string(),
            public: false,
            secret: None,
        },
        &[],
    )?;

    let res = client.get("/api/dashboard/projects").send().await;
    res.assert_json(json!({"projects": []})).await;

    app.users.create("test", "test", UserRole::User, &[])?;
    app.users.create("test2", "test", UserRole::User, &["private-project"])?;

    let login1 = common::login(&client, "test", "test").await;
    let login2 = common::login(&client, "test2", "test").await;

    let res = client.get("/api/dashboard/projects").header(header::COOKIE, common::cookie_header(&login1)).send().await;
    res.assert_json(json!({"projects": []})).await;

    let res = client.get("/api/dashboard/projects").header(header::COOKIE, common::cookie_header(&login2)).send().await;
    res.assert_json(json!({"projects": [{"displayName": "Private Project", "id": "private-project", "public": false, "entities": []}]})).await;

    Ok(())
}
