use anyhow::Result;
use chrono::{Duration, Utc};
use liwan::{
    app::{
        Liwan,
        models::{self, UserRole},
    },
    config::Config,
};
use serde_json::json;

mod common;

#[tokio::test]
async fn test_login() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.users.create("test", "testtesttesttest", UserRole::User, &[])?;

    // login
    let login = json!({ "username": "test", "password": "testtesttesttest" });
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

    let token = app.onboarding.token().expect("onboarding should exist");

    // Invalid token should return 401
    let setup = json!({ "token": "invalid_token", "username": "admin2", "password": "adminadminadmin" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_unauthorized();

    // Valid token should return 200
    let setup = json!({ "token": token, "username": "admin", "password": "adminadminadmin" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_success();

    // Check that the user is created
    let login = json!({ "username": "admin", "password": "adminadminadmin" });
    let res = client.post("/api/dashboard/auth/login", login).await;
    res.assert_status_success();

    // Check that the onboarding is cleared
    assert_eq!(app.onboarding.token(), None, "onboarding should be cleared");
    let setup = json!({ "token": token, "username": "admin", "password": "adminadminadmin" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_unauthorized();

    let setup = json!({ "token": token, "username": "admin2", "password": "adminadminadmin2" });
    let res = client.post("/api/dashboard/auth/setup", setup).await;
    res.assert_status_unauthorized();

    Ok(())
}

#[tokio::test]
async fn expired_session() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.users.create("test", "testtesttesttest", UserRole::User, &[])?;

    // login
    let login = json!({ "username": "test", "password": "testtesttesttest" });
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
async fn session_past_its_expiration_is_rejected() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.users.create("test", "testtesttesttest", UserRole::User, &[])?;
    app.sessions.create("expired-session", "test", Utc::now() - Duration::minutes(1))?;

    let res = client
        .get_with_headers(
            "/api/dashboard/auth/me",
            vec![("cookie".to_string(), "liwan-session=expired-session; liwan-username=test".to_string())],
        )
        .await;
    res.assert_status_unauthorized();

    let cookies = common::cookies(&res);
    assert!(cookies.iter().any(|cookie| cookie.name() == "liwan-session"));
    assert!(cookies.iter().any(|cookie| cookie.name() == "liwan-username"));
    Ok(())
}

#[tokio::test]
async fn authentication_cookie_boundaries() -> Result<()> {
    let mut config = Config::default();
    config.base_url = "https://liwan.example.com".to_string();
    let app = Liwan::new_memory(config)?;
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.users.create("test", "testtest", UserRole::User, &[])?;
    let cookies = common::login(&client, "test", "testtest").await;
    let session = cookies.iter().find(|cookie| cookie.name() == "liwan-session").expect("session cookie");
    let username = cookies.iter().find(|cookie| cookie.name() == "liwan-username").expect("username cookie");

    assert_eq!(session.path(), Some("/api/dashboard"));
    assert_eq!(session.http_only(), Some(true));
    assert_eq!(session.secure(), Some(true));
    assert_eq!(session.same_site(), Some(cookie::SameSite::Strict));
    assert_eq!(username.path(), Some("/"));
    assert!(!username.http_only().unwrap_or(false));
    assert_eq!(username.secure(), Some(true));
    assert_eq!(username.same_site(), Some(cookie::SameSite::Strict));

    let res = client
        .get_with_headers("/api/dashboard/auth/me", vec![("cookie".to_string(), "liwan-username=test".to_string())])
        .await;
    res.assert_status_unauthorized();
    assert_eq!(common::cookies(&res).len(), 2);

    let session_id = session.value().to_string();
    let res = client
        .get_with_headers("/api/dashboard/auth/me", vec![("cookie".to_string(), format!("liwan-session={session_id}"))])
        .await;
    res.assert_status_unauthorized();
    assert!(app.sessions.get(&session_id)?.is_none());

    let cookies = common::login(&client, "test", "testtest").await;
    let session = cookies.iter().find(|cookie| cookie.name() == "liwan-session").expect("session cookie");
    let res = client
        .get_with_headers(
            "/api/dashboard/auth/me",
            vec![("cookie".to_string(), format!("liwan-session={}; liwan-username=forged", session.value()))],
        )
        .await;
    res.assert_json(&json!({ "username": "test", "role": "user" }));

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
            unlisted: false,
            secret: None,
        },
        &[],
    )?;

    let res = client.get("/api/dashboard/projects").await;
    res.assert_json(&json!({"projects": []}));

    app.users.create("test", "testtesttesttest", UserRole::User, &[])?;
    app.users.create("test2", "test", UserRole::User, &["private-project"])?;

    let login1 = common::login(&client, "test", "testtesttesttest").await;
    let login2 = common::login(&client, "test2", "test").await;

    let res = client
        .get_with_headers("/api/dashboard/projects", vec![("cookie".to_string(), common::cookie_header(&login1))])
        .await;
    res.assert_json(&json!({"projects": []}));

    let res = client
        .get_with_headers("/api/dashboard/projects", vec![("cookie".to_string(), common::cookie_header(&login2))])
        .await;
    res.assert_json(&json!({"projects": [{"displayName": "Private Project", "id": "private-project", "public": false, "unlisted": false, "entities": [], "hiddenMetrics": [], "hiddenDimensions": []}]}));

    Ok(())
}

#[tokio::test]
async fn private_project_reports_require_access() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.projects.create(
        &models::Project {
            display_name: "Private Project".to_string(),
            id: "private-project".to_string(),
            public: false,
            unlisted: false,
            secret: None,
        },
        &[],
    )?;
    app.users.create("unassigned", "testtest", UserRole::User, &[])?;
    app.users.create("assigned", "testtest", UserRole::User, &["private-project"])?;

    let unassigned = common::login(&client, "unassigned", "testtest").await;
    let assigned = common::login(&client, "assigned", "testtest").await;
    let unassigned_header = vec![("cookie".to_string(), common::cookie_header(&unassigned))];
    let assigned_header = vec![("cookie".to_string(), common::cookie_header(&assigned))];
    let prefix = "/api/dashboard/project/private-project";
    let start = (Utc::now() - Duration::hours(1)).to_rfc3339();
    let end = Utc::now().to_rfc3339();
    let reports = [
        (
            format!("{prefix}/graph"),
            json!({"range":{"start":start,"end":end},"filters":[],"interval":"hour","timezone":"UTC","metric":"views"}),
        ),
        (format!("{prefix}/stats"), json!({"range":{"start":start,"end":end},"filters":[]})),
        (
            format!("{prefix}/dimension"),
            json!({"range":{"start":start,"end":end},"filters":[],"metric":"views","dimension":"url"}),
        ),
    ];

    client.get(&format!("{prefix}/earliest")).await.assert_status_not_found();
    client.get_with_headers(&format!("{prefix}/earliest"), unassigned_header.clone()).await.assert_status_not_found();
    client.get_with_headers(&format!("{prefix}/earliest"), assigned_header.clone()).await.assert_status_success();

    for (path, body) in reports {
        client.post(&path, body.clone()).await.assert_status_not_found();
        client.post_with_headers(&path, body.clone(), unassigned_header.clone()).await.assert_status_not_found();
        client.post_with_headers(&path, body, assigned_header.clone()).await.assert_status_success();
    }

    Ok(())
}
