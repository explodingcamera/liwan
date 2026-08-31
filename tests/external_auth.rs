use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, routing::get};
use liwan::{
    app::{Liwan, models::UserRole},
    config::Config,
};
use serde_json::{Value, json};

mod common;

fn settings(overrides: Value) -> Value {
    let mut settings = json!({
        "enabled": false,
        "provider": "oidc",
        "displayName": "OpenID Connect",
        "clientId": "client-id",
        "issuerUrl": null,
        "allowedDomain": null,
        "tenantId": null,
        "allowUserCreation": false,
        "allowSessionReuse": true
    });
    settings.as_object_mut().unwrap().extend(overrides.as_object().unwrap().clone());
    settings
}

async fn authenticated_client(role: UserRole) -> Result<(Arc<Liwan>, common::TestClient, String)> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.users.create("test-user", "test-password", role, &[])?;
    app.onboarding.clear()?;
    let cookies = common::login(&client, "test-user", "test-password").await;
    Ok((app, client, common::cookie_header(&cookies)))
}

#[tokio::test]
async fn public_metadata_is_disabled_during_onboarding() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app, tx);

    let response = client.get("/api/dashboard/auth/external").await;
    response.assert_json(&json!({
        "enabled": false,
        "provider": "oidc",
        "displayName": "OpenID Connect"
    }));

    let response = client.get("/api/dashboard/auth/external/start").await;
    response.assert_status_not_found();
    Ok(())
}

#[tokio::test]
async fn settings_require_an_admin() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app, tx);
    client.get("/api/dashboard/admin/auth").await.assert_status_unauthorized();

    let (_app, client, cookies) = authenticated_client(UserRole::User).await?;
    let response = client.get_with_headers("/api/dashboard/admin/auth", vec![("cookie".to_string(), cookies)]).await;
    response.assert_status_forbidden();
    Ok(())
}

#[tokio::test]
async fn settings_redact_preserve_and_clear_the_secret() -> Result<()> {
    let (app, client, cookies) = authenticated_client(UserRole::Admin).await?;
    let headers = || vec![("cookie".to_string(), cookies.clone())];

    let response = client
        .put_with_headers(
            "/api/dashboard/admin/auth",
            settings(json!({
                "displayName": "Company login",
                "clientSecret": "top-secret",
                "issuerUrl": "https://accounts.example.com"
            })),
            headers(),
        )
        .await;
    response.assert_status_success();
    let body: Value = response.json();
    assert_eq!(body["clientSecretConfigured"], true);
    assert!(body.get("clientSecret").is_none());
    assert_eq!(app.external_auth.settings()?.client_secret.as_deref(), Some("top-secret"));

    let response = client
        .put_with_headers(
            "/api/dashboard/admin/auth",
            settings(json!({
                "displayName": "Renamed login",
                "issuerUrl": "https://accounts.example.com"
            })),
            headers(),
        )
        .await;
    response.assert_status_success();
    assert_eq!(app.external_auth.settings()?.client_secret.as_deref(), Some("top-secret"));

    let response = client
        .put_with_headers(
            "/api/dashboard/admin/auth",
            settings(json!({
                "displayName": "Renamed login",
                "clearClientSecret": true,
                "issuerUrl": "https://accounts.example.com"
            })),
            headers(),
        )
        .await;
    response.assert_status_success();
    assert_eq!(response.json::<Value>()["clientSecretConfigured"], false);
    assert_eq!(app.external_auth.settings()?.client_secret, None);
    Ok(())
}

#[tokio::test]
async fn invalid_enabled_settings_are_not_persisted() -> Result<()> {
    let (app, client, cookies) = authenticated_client(UserRole::Admin).await?;
    let response = client
        .put_with_headers(
            "/api/dashboard/admin/auth",
            settings(json!({ "enabled": true, "clientId": "" })),
            vec![("cookie".to_string(), cookies)],
        )
        .await;
    response.assert_status_bad_request();
    assert!(!app.external_auth.settings()?.enabled);
    Ok(())
}

#[tokio::test]
async fn oidc_start_uses_discovery_pkce_and_state_cookie() -> Result<()> {
    let (issuer, server) = mock_oidc_provider().await?;
    let mut config = Config::default();
    config.base_url = "https://liwan.example.com".to_string();
    let app = Liwan::new_memory(config)?;
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.users.create("admin", "test-password", UserRole::Admin, &[])?;
    app.onboarding.clear()?;
    let cookies = common::login(&client, "admin", "test-password").await;
    let cookie_header = common::cookie_header(&cookies);

    let response = client
        .put_with_headers(
            "/api/dashboard/admin/auth",
            settings(json!({
                "enabled": true,
                "allowSessionReuse": false,
                "clientSecret": "secret",
                "issuerUrl": issuer
            })),
            vec![("cookie".to_string(), cookie_header)],
        )
        .await;
    response.assert_status_success();
    assert_eq!(
        response.json::<Value>()["callbackUrl"],
        "https://liwan.example.com/api/dashboard/auth/external/callback"
    );

    let response = client.get("/api/dashboard/auth/external").await;
    assert_eq!(response.json::<Value>()["enabled"], true);

    let response = client.get("/api/dashboard/auth/external/start?returnTo=%2Fsettings").await;
    response.assert_status_see_other();
    let location = response.header("location").to_str()?.to_string();
    let authorization_url = url::Url::parse(&location)?;
    let query: std::collections::HashMap<_, _> = authorization_url.query_pairs().into_owned().collect();
    assert_eq!(
        query.get("redirect_uri").map(String::as_str),
        Some("https://liwan.example.com/api/dashboard/auth/external/callback")
    );
    assert_eq!(query.get("code_challenge_method").map(String::as_str), Some("S256"));
    assert!(query.contains_key("code_challenge"));
    assert!(query.contains_key("nonce"));
    assert_eq!(query.get("prompt").map(String::as_str), Some("login"));
    let state = query.get("state").expect("state query parameter");

    let state_cookie = common::cookies(&response)
        .into_iter()
        .find(|cookie| cookie.name() == "liwan-external-auth-state")
        .expect("state cookie");
    assert_eq!(state_cookie.value(), state);
    assert!(state_cookie.http_only().unwrap_or(false));
    assert!(state_cookie.secure().unwrap_or(false));
    assert_eq!(state_cookie.same_site(), Some(cookie::SameSite::Lax));

    let callback = client
        .get_with_headers(
            "/api/dashboard/auth/external/callback?state=wrong&code=unused",
            vec![("cookie".to_string(), format!("{}={}", state_cookie.name(), state_cookie.value()))],
        )
        .await;
    callback.assert_status_see_other();
    callback.assert_header("location", "/login?externalAuthError=1");
    assert!(common::cookies(&callback).into_iter().all(|cookie| cookie.name() != "liwan-external-auth-state"));

    let response = client.get("/api/dashboard/auth/external/start?returnTo=https%3A%2F%2Fevil.example").await;
    response.assert_status_bad_request();
    server.abort();
    Ok(())
}

async fn mock_oidc_provider() -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let issuer = format!("http://{}", listener.local_addr()?);
    let metadata_issuer = issuer.clone();
    let router = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(move || {
                let issuer = metadata_issuer.clone();
                async move {
                    Json(json!({
                        "issuer": issuer,
                        "authorization_endpoint": format!("{issuer}/authorize"),
                        "token_endpoint": format!("{issuer}/token"),
                        "jwks_uri": format!("{issuer}/jwks"),
                        "response_types_supported": ["code"],
                        "subject_types_supported": ["public"],
                        "id_token_signing_alg_values_supported": ["RS256"],
                        "token_endpoint_auth_methods_supported": ["client_secret_basic"]
                    }))
                }
            }),
        )
        .route("/jwks", get(|| async { Json(json!({ "keys": [] })) }));
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    Ok((issuer, server))
}
