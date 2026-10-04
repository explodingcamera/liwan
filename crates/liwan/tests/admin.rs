use anyhow::Result;
use chrono::{Duration, Utc};
use liwan::app::models::{
    AccessPermission, DataRetention, Entity, EntityCollectionSettings, Event, GeoDetail, Project, ProjectVisibility,
    UserRole,
};
use serde_json::{Value, json};
use std::num::NonZeroU32;

mod common;

#[tokio::test]
async fn entities_list_respects_team_access() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.users.create("admin", "testtest", UserRole::Admin)?;
    app.users.create("viewer", "testtest", UserRole::User)?;
    app.entities.create(&Entity { id: "allowed".into(), display_name: "Allowed".into() }, &[])?;
    app.entities.create(&Entity { id: "hidden".into(), display_name: "Hidden".into() }, &[])?;
    app.projects.create(
        &Project {
            id: "visible".into(),
            display_name: "Visible".into(),
            visibility: ProjectVisibility::Private,
            secret: None,
        },
        &["allowed".into()],
    )?;
    app.projects.create(
        &Project {
            id: "unlisted".into(),
            display_name: "Unlisted".into(),
            visibility: ProjectVisibility::Unlisted,
            secret: None,
        },
        &["allowed".into(), "hidden".into()],
    )?;
    let team_id = app.teams.create("Viewers")?;
    let admin = common::login(&client, "admin", "testtest").await;
    client
        .put_with_headers(
            &format!("/api/dashboard/team/{team_id}"),
            json!({"displayName": "Viewers", "users": ["viewer"], "projects": ["visible"]}),
            vec![("cookie".into(), common::cookie_header(&admin))],
        )
        .await
        .assert_status_success();

    client.get("/api/dashboard/entities").await.assert_status(http::StatusCode::UNAUTHORIZED);
    let admin_headers = vec![("cookie".into(), common::cookie_header(&admin))];
    let admin_list: Value = client.get_with_headers("/api/dashboard/entities", admin_headers).await.json();
    assert_eq!(admin_list["entities"].as_array().unwrap().len(), 2);
    let viewer = common::login(&client, "viewer", "testtest").await;
    let headers = || vec![("cookie".into(), common::cookie_header(&viewer))];
    let listed: Value = client.get_with_headers("/api/dashboard/entities", headers()).await.json();
    assert_eq!(
        listed["entities"],
        json!([{"id": "allowed", "displayName": "Allowed", "projects": [{"id": "visible", "displayName": "Visible", "visibility": "private"}]}])
    );
    Ok(())
}

#[tokio::test]
async fn teams_grant_project_read_access() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.users.create("admin", "testtest", UserRole::Admin)?;
    app.users.create("viewer", "testtest", UserRole::User)?;
    app.projects.create(
        &Project {
            id: "private".into(),
            display_name: "Private".into(),
            visibility: ProjectVisibility::Private,
            secret: None,
        },
        &[],
    )?;
    let admin = common::login(&client, "admin", "testtest").await;
    let viewer = common::login(&client, "viewer", "testtest").await;
    let admin_headers = || vec![("cookie".into(), common::cookie_header(&admin))];
    let viewer_headers = || vec![("cookie".into(), common::cookie_header(&viewer))];

    client.get_with_headers("/api/dashboard/teams", viewer_headers()).await.assert_status_forbidden();
    client
        .get_with_headers("/api/dashboard/project/private", viewer_headers())
        .await
        .assert_status(http::StatusCode::NOT_FOUND);
    let created: Value = client
        .post_with_headers("/api/dashboard/teams", json!({"displayName": "Engineering"}), admin_headers())
        .await
        .json();
    let team_id = created["id"].as_str().unwrap();
    assert_eq!(team_id.len(), 16);
    client
        .put_with_headers(
            &format!("/api/dashboard/team/{team_id}"),
            json!({"displayName": "Engineering", "users": ["viewer"], "projects": ["private"]}),
            admin_headers(),
        )
        .await
        .assert_status_success();
    let teams: Value = client.get_with_headers("/api/dashboard/teams", admin_headers()).await.json();
    assert_eq!(teams["teams"][0]["id"], team_id);
    assert_eq!(teams["teams"][0]["entities"], json!([]));
    assert_eq!(teams["teams"][0]["projects"], json!(["private"]));
    client.get_with_headers("/api/dashboard/project/private", viewer_headers()).await.assert_status_success();
    client
        .put_with_headers("/api/dashboard/user/viewer", json!({"role": "admin", "teams": ["missing"]}), admin_headers())
        .await
        .assert_status(http::StatusCode::BAD_REQUEST);
    assert_eq!(app.users.get("viewer")?.role, UserRole::User);
    client
        .put_with_headers("/api/dashboard/user/viewer", json!({"role": "user", "teams": []}), admin_headers())
        .await
        .assert_status_success();
    client
        .get_with_headers("/api/dashboard/project/private", viewer_headers())
        .await
        .assert_status(http::StatusCode::NOT_FOUND);
    client
        .put_with_headers("/api/dashboard/user/viewer", json!({"role": "user", "teams": [team_id]}), admin_headers())
        .await
        .assert_status_success();
    client.get_with_headers("/api/dashboard/project/private", viewer_headers()).await.assert_status_success();
    let listed: Value = client.get_with_headers("/api/dashboard/projects", viewer_headers()).await.json();
    assert_eq!(listed["projects"][0]["id"], "private");
    client
        .delete_with_headers(&format!("/api/dashboard/team/{team_id}"), admin_headers())
        .await
        .assert_status_success();
    client
        .get_with_headers("/api/dashboard/project/private", viewer_headers())
        .await
        .assert_status(http::StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn internal_projects_require_a_session() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.users.create("viewer", "testtest", UserRole::User)?;
    app.projects.create(
        &Project {
            id: "staff".into(),
            display_name: "Staff".into(),
            visibility: ProjectVisibility::Internal,
            secret: None,
        },
        &[],
    )?;
    client.get("/api/dashboard/project/staff").await.assert_status(http::StatusCode::NOT_FOUND);
    let listed: Value = client.get("/api/dashboard/projects").await.json();
    assert_eq!(listed["projects"], json!([]));
    let cookies = common::login(&client, "viewer", "testtest").await;
    let headers = || vec![("cookie".into(), common::cookie_header(&cookies))];
    client.get_with_headers("/api/dashboard/project/staff", headers()).await.assert_status_success();
    let listed: Value = client.get_with_headers("/api/dashboard/projects", headers()).await.json();
    assert_eq!(listed["projects"][0]["id"], "staff");
    assert_eq!(listed["projects"][0]["visibility"], "internal");
    Ok(())
}

#[tokio::test]
async fn admin_manages_api_keys() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.users.create("admin", "testtest", UserRole::Admin)?;
    app.entities.create(&Entity { id: "service".into(), display_name: "Service".into() }, &[])?;
    app.entities.create(&Entity { id: "other".into(), display_name: "Other".into() }, &[])?;
    let cookies = common::login(&client, "admin", "testtest").await;
    let headers = || vec![("cookie".to_string(), common::cookie_header(&cookies))];

    client
        .post_with_headers(
            "/api/dashboard/api-keys",
            json!({ "displayName": "Unsupported", "entities": [], "projects": [], "permissions": ["project:read"], "expiration": "never" }),
            headers(),
        )
        .await
        .assert_status_bad_request();

    let created = client
        .post_with_headers(
            "/api/dashboard/api-keys",
            json!({ "displayName": "Production", "entities": ["service"], "projects": [], "permissions": ["events:batch"], "expiration": "never" }),
            headers(),
        )
        .await;
    created.assert_status(http::StatusCode::CREATED);
    let created: Value = created.json();
    let plaintext = created["plaintext"].as_str().expect("plaintext key");
    assert!(plaintext.starts_with("liw_"));
    let key_id = created["key"]["id"].as_str().expect("key ID");

    let listed = client.get_with_headers("/api/dashboard/api-keys", headers()).await;
    listed.assert_status_success();
    assert_eq!(listed.header("cache-control"), "private, no-store");
    let listed: Value = listed.json();
    assert_eq!(listed["keys"].as_array().unwrap().len(), 1);
    assert!(listed.to_string().find(plaintext).is_none());

    app.users.create("viewer", "testtest", UserRole::User)?;
    let viewer_cookies = common::login(&client, "viewer", "testtest").await;
    client
        .post_with_headers(
            &format!("/api/dashboard/api-keys/{key_id}/regenerate"),
            json!({"expiration": "never"}),
            vec![("cookie".to_string(), common::cookie_header(&viewer_cookies))],
        )
        .await
        .assert_status_forbidden();
    assert!(app.api_keys.authenticate(plaintext)?.is_some());

    client
        .put_with_headers(
            &format!("/api/dashboard/api-keys/{key_id}"),
            json!({ "displayName": "Production API", "entities": ["other"], "projects": [], "permissions": [] }),
            headers(),
        )
        .await
        .assert_status_success();
    let access = app.api_keys.authenticate(plaintext)?.expect("valid API key");
    assert!(!access.access.can_access_entity("service", AccessPermission::EventsBatch));
    assert!(access.access.entities.contains("other"));
    assert!(!access.access.can_access_entity("other", AccessPermission::EventsBatch));
    let regenerated = client
        .post_with_headers(
            &format!("/api/dashboard/api-keys/{key_id}/regenerate"),
            json!({"expiration": "7_days"}),
            headers(),
        )
        .await;
    regenerated.assert_status_success();
    let regenerated: Value = regenerated.json();
    assert_eq!(regenerated["key"]["id"], key_id);
    assert!(regenerated["key"]["expiresAt"].is_string());
    assert!(app.api_keys.authenticate(plaintext)?.is_none());
    let replacement = regenerated["plaintext"].as_str().unwrap();
    assert!(app.api_keys.authenticate(replacement)?.is_some());

    client.delete_with_headers(&format!("/api/dashboard/api-keys/{key_id}"), headers()).await.assert_status_success();
    assert!(app.api_keys.authenticate(plaintext)?.is_none());
    assert!(app.api_keys.authenticate(replacement)?.is_none());
    Ok(())
}

#[tokio::test]
async fn admin_cannot_delete_self_using_different_casing() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.users.create("admin", "testtest", UserRole::Admin)?;
    let cookies = common::login(&client, "admin", "testtest").await;
    let headers = vec![("cookie".to_string(), common::cookie_header(&cookies))];

    client.delete_with_headers("/api/dashboard/user/Admin", headers.clone()).await.assert_status_forbidden();
    client.get_with_headers("/api/dashboard/auth/me", headers).await.assert_status_success();
    common::login(&client, "admin", "testtest").await;

    Ok(())
}

#[tokio::test]
async fn relationship_updates_are_atomic() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.users.create("admin", "testtest", UserRole::Admin)?;
    let cookies = common::login(&client, "admin", "testtest").await;
    let headers = || vec![("cookie".to_string(), common::cookie_header(&cookies))];

    app.entities.create(&Entity { id: "existing-entity".into(), display_name: "Existing entity".into() }, &[])?;
    client
        .post_with_headers(
            "/api/dashboard/project/failed-project",
            json!({
                "displayName": "Failed project",
                "visibility": "private",
                "secret": null,
                "entities": ["existing-entity", "missing-entity"]
            }),
            headers(),
        )
        .await
        .assert_status_bad_request();
    assert!(app.projects.get("failed-project").is_err());

    app.projects.create(
        &Project {
            id: "existing-project".into(),
            display_name: "Existing project".into(),
            visibility: ProjectVisibility::Private,
            secret: None,
        },
        &[],
    )?;
    client
        .post_with_headers(
            "/api/dashboard/entity",
            json!({
                "id": "failed-entity",
                "displayName": "Failed entity",
                "projects": ["existing-project", "missing-project"]
            }),
            headers(),
        )
        .await
        .assert_status_bad_request();
    assert!(!app.entities.exists("failed-entity")?);

    app.entities.create(&Entity { id: "new-entity".into(), display_name: "New entity".into() }, &[])?;
    app.projects.create(
        &Project {
            id: "updated-project".into(),
            display_name: "Updated project".into(),
            visibility: ProjectVisibility::Private,
            secret: None,
        },
        &["existing-entity".to_string()],
    )?;
    client
        .put_with_headers(
            "/api/dashboard/project/updated-project",
            json!({ "entities": ["new-entity", "missing-entity"] }),
            headers(),
        )
        .await
        .assert_status_bad_request();
    assert_eq!(app.projects.entity_ids("updated-project")?, vec!["existing-entity"]);

    app.projects.create(
        &Project {
            id: "new-project".into(),
            display_name: "New project".into(),
            visibility: ProjectVisibility::Private,
            secret: None,
        },
        &[],
    )?;
    app.entities.create(
        &Entity { id: "updated-entity".into(), display_name: "Updated entity".into() },
        &["existing-project".to_string()],
    )?;
    client
        .put_with_headers(
            "/api/dashboard/entity/updated-entity",
            json!({ "projects": ["new-project", "missing-project"] }),
            headers(),
        )
        .await
        .assert_status_bad_request();
    let projects = app.entities.projects("updated-entity")?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, "existing-project");

    Ok(())
}

#[tokio::test]
async fn pruning_supports_dry_run_and_is_idempotent() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);
    app.users.create("admin", "testtest", UserRole::Admin)?;
    let cookies = common::login(&client, "admin", "testtest").await;
    let headers = || vec![("cookie".to_string(), common::cookie_header(&cookies))];

    for id in ["pruned-entity", "preserved-entity"] {
        app.entities.create(&Entity { id: id.into(), display_name: id.into() }, &[])?;
    }
    app.settings.update_entity(&EntityCollectionSettings {
        entity_id: "pruned-entity".into(),
        visitor_group_mode: None,
        track_sessions: Some(false),
        track_utm_params: Some(false),
        track_geo: Some(GeoDetail::None),
        data_retention: DataRetention::Days(NonZeroU32::new(30).unwrap()),
        allowed_hostnames: Vec::new(),
        ingest_drop_rules: Vec::new(),
    })?;

    let now = Utc::now();
    let event = |entity_id: &str, visitor_group_id: &str, created_at: chrono::DateTime<Utc>, sensitive: bool| Event {
        entity_id: entity_id.into(),
        visitor_group_id: visitor_group_id.into(),
        event: "pageview".into(),
        created_at,
        fqdn: Some("example.com".into()),
        path: Some("/".into()),
        referrer: None,
        platform: None,
        browser: None,
        mobile: None,
        country: sensitive.then(|| "AU".into()),
        city: sensitive.then(|| "Sydney".into()),
        utm_source: sensitive.then(|| "newsletter".into()),
        utm_medium: None,
        utm_campaign: None,
        utm_content: None,
        utm_term: None,
        screen_width: None,
        orientation: None,
        properties: Default::default(),
        track_sessions: true,
    };
    app.events.append(
        vec![
            event("pruned-entity", "old", now - Duration::days(60), false),
            event("pruned-entity", "recent", now - Duration::minutes(2), true),
            event("pruned-entity", "recent", now - Duration::minutes(1), true),
            event("preserved-entity", "preserved", now - Duration::minutes(2), true),
            event("preserved-entity", "preserved", now - Duration::minutes(1), true),
        ]
        .into_iter(),
    )?;

    let state = |entity_id: &str| -> Result<(u64, u64, u64, u64)> {
        Ok(app.events_conn()?.query_row(
            "select count(*), count(*) filter (where utm_source is not null), count(*) filter (where country is not null or city is not null), count(*) filter (where time_from_last_event is not null or time_to_next_event is not null) from events where entity_id = ?",
            duckdb::params![entity_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?)
    };
    assert_eq!(state("pruned-entity")?, (3, 2, 2, 2));

    let response =
        client.post_with_headers("/api/dashboard/settings/prune", json!({ "dryRun": true }), headers()).await;
    response.assert_status_success();
    let body: Value = response.json();
    assert_eq!(
        prune_stats(&body, "pruned-entity"),
        &json!({
            "entityId": "pruned-entity",
            "totalEvents": 3,
            "deletedEvents": 1,
            "clearedUtmEvents": 2,
            "clearedGeoEvents": 2,
            "clearedSessionEvents": 2
        })
    );
    assert_eq!(state("pruned-entity")?, (3, 2, 2, 2));

    let response =
        client.post_with_headers("/api/dashboard/settings/prune", json!({ "dryRun": false }), headers()).await;
    response.assert_status_success();
    assert_eq!(state("pruned-entity")?, (2, 0, 0, 0));
    assert_eq!(state("preserved-entity")?, (2, 2, 2, 2));

    let response =
        client.post_with_headers("/api/dashboard/settings/prune", json!({ "dryRun": false }), headers()).await;
    response.assert_status_success();
    let body: Value = response.json();
    let stats = prune_stats(&body, "pruned-entity");
    assert_eq!(stats["deletedEvents"], 0);
    assert_eq!(stats["clearedUtmEvents"], 0);
    assert_eq!(stats["clearedGeoEvents"], 0);
    assert_eq!(stats["clearedSessionEvents"], 0);

    Ok(())
}

fn prune_stats<'a>(body: &'a Value, entity_id: &str) -> &'a Value {
    body["entities"]
        .as_array()
        .and_then(|entities| entities.iter().find(|entity| entity["entityId"] == entity_id))
        .expect("entity pruning stats")
}
