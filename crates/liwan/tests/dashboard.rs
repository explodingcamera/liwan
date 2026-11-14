mod common;
use anyhow::Result;
use chrono::{Duration, Utc};
use liwan::app::models::ProjectVisibility;
use serde_json::json;

#[tokio::test]
async fn graph_preserves_partial_buckets() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.seed_database(0)?;
    for (time, visitor, seconds) in [
        ("08:39:59", "outside", 100),
        ("08:40:00", "a", 10),
        ("09:00:00", "a", 30),
        ("09:29:59", "b", 80),
        ("09:30:00", "a", 60),
        ("10:05:00", "a", 120),
        ("10:10:00", "outside", 100),
    ] {
        let date = format!("2025-01-01T{time}Z").parse::<chrono::DateTime<Utc>>()?;
        app.events_conn()?.execute(
            "insert into events (entity_id, visitor_group_id, event, created_at, exited_at) values ('entity-1', ?, 'pageview', ?, ?)",
            duckdb::params![visitor, date, date + Duration::seconds(seconds)],
        )?;
    }
    for (metric, expected) in [
        ("views", json!([1.0, 3.0, 1.0])),
        ("unique_visitors", json!([1.0, 2.0, 1.0])),
        ("avg_time_on_site", json!([10.0, 170.0 / 3.0, 120.0])),
        ("bounce_rate", json!([1.0, 1.0, 1.0])),
    ] {
        let response = client
            .post(
                "/api/dashboard/project/public-project/graph",
                json!({
                    "ranges": [
                        { "start": "2026-01-01T10:10:00Z", "end": "2026-01-01T11:40:00Z" },
                        { "start": "2025-01-01T08:40:00Z", "end": "2025-01-01T10:10:00Z" },
                    ],
                    "filters": [], "metric": metric, "interval": "hour", "timezone": "UTC",
                }),
            )
            .await;
        response.assert_status_success();
        let body: serde_json::Value = response.json();
        let rows = body["reports"][1].as_array().unwrap();
        assert_eq!(json!(rows.iter().map(|row| &row["value"]).collect::<Vec<_>>()), expected, "{metric}");
        assert_eq!(rows[0]["binStart"], "2025-01-01T08:00:00Z");
        assert_eq!(rows[1]["binStart"], "2025-01-01T09:00:00Z");
        assert_eq!(rows[2]["binStart"], "2025-01-01T10:00:00Z");
        assert_eq!(body["reports"][0].as_array().unwrap().len(), 2);
    }

    Ok(())
}

#[tokio::test]
async fn reports_follow_range_order() -> Result<()> {
    let mut config = liwan::config::Config::default();
    config.limits.report_max_concurrency = 1;
    let app = liwan::app::Liwan::new_memory(config)?;
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.seed_database(0)?;

    for day in 1..=2 {
        let date = format!("2026-01-0{day}T12:00:00Z").parse::<chrono::DateTime<Utc>>()?;
        for (event, path, count) in
            [("signup", "/included", day), ("signup", "/excluded", 5), ("pageview", "/included", 5)]
        {
            for _ in 0..count {
                app.events_conn()?.execute(
                    "insert into events (entity_id, visitor_group_id, event, created_at, path) values (?, ?, ?, ?, ?)",
                    duckdb::params!["entity-1", "visitor", event, date, path],
                )?;
            }
        }
    }

    // Reverse chronological order, followed by an empty range and an overlapping range.
    let ranges = json!([
        { "start": "2026-01-02T00:00:00Z", "end": "2026-01-03T00:00:00Z" },
        { "start": "2026-01-01T00:00:00Z", "end": "2026-01-02T00:00:00Z" },
        { "start": "2026-01-03T00:00:00Z", "end": "2026-01-04T00:00:00Z" },
        { "start": "2026-01-01T12:00:00Z", "end": "2026-01-02T13:00:00Z" },
    ]);
    for endpoint in ["stats", "graph", "dimension", "custom-events"] {
        let response = client
            .post(
                &format!("/api/dashboard/project/public-project/{endpoint}"),
                json!({
                    "ranges": ranges,
                    "filters": [{ "dimension": "path", "filterType": "equal", "value": "/included" }],
                    "event": "signup", "metric": "views", "dimension": "path", "interval": "day", "timezone": "UTC",
                }),
            )
            .await;
        response.assert_status_success();
        let body: serde_json::Value = response.json();
        let reports = body["reports"].as_array().expect("reports should be an array");
        assert_eq!(reports.len(), 4);
        for (report, expected) in reports.iter().zip([2.0, 1.0, 0.0, 3.0]) {
            let value = match endpoint {
                "stats" => report["totalViews"].as_f64().unwrap(),
                "graph" | "dimension" => {
                    report.as_array().unwrap().iter().filter_map(|row| row["value"].as_f64()).sum()
                }
                _ => report["rows"].as_array().unwrap().iter().filter_map(|row| row["completions"].as_f64()).sum(),
            };
            assert_eq!(value, expected, "{endpoint}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn range_limits_and_samples_are_independent() -> Result<()> {
    let mut config = liwan::config::Config::default();
    config.limits.report_max_dimension_results = 1;
    config.limits.report_min_bounce_rate_sessions = 2;
    config.limits.report_min_avg_time_on_site_samples = 2;
    let app = liwan::app::Liwan::new_memory(config)?;
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.seed_database(0)?;
    for (day, path, count, seconds) in
        [(1, "/a", 2, 20), (1, "/b", 3, 60), (2, "/a", 1, 100), (2, "/b", 2, 50), (3, "/a", 1, 10)]
    {
        let date = format!("2026-01-0{day}T12:00:00Z").parse::<chrono::DateTime<Utc>>()?;
        for _ in 0..count {
            app.events_conn()?.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at, path, exited_at) values ('entity-1', 'visitor', 'pageview', ?, ?, ?)",
                duckdb::params![date, path, date + Duration::seconds(seconds)],
            )?;
        }
    }
    for (day, event, count) in
        [(1, "signup", 3), (1, "download", 1), (1, "purchase", 1), (2, "signup", 1), (2, "download", 2)]
    {
        let date = format!("2026-01-0{day}T12:00:00Z").parse::<chrono::DateTime<Utc>>()?;
        for _ in 0..count {
            app.events_conn()?.execute(
                "insert into events (entity_id, visitor_group_id, event, created_at) values ('entity-1', 'visitor', ?, ?)",
                duckdb::params![event, date],
            )?;
        }
    }
    let ranges = json!([
        { "start": "2026-01-01T00:00:00Z", "end": "2026-01-02T00:00:00Z" },
        { "start": "2026-01-02T00:00:00Z", "end": "2026-01-03T00:00:00Z" },
        { "start": "2026-01-03T00:00:00Z", "end": "2026-01-04T00:00:00Z" },
    ]);
    for (metric, expected) in [
        ("views", json!([[["/a", 2.0], ["/b", 3.0]], [["/a", 1.0], ["/b", 2.0]], [["/a", 1.0], ["/b", null]]])),
        ("bounce_rate", json!([[["/a", 1.0], ["/b", 1.0]], [["/a", null], ["/b", 1.0]], [["/a", null], ["/b", null]]])),
        ("avg_time_on_site", json!([[["/b", 60.0]], [["/b", 50.0]], [["/b", null]]])),
    ] {
        let response = client
            .post(
                "/api/dashboard/project/public-project/dimension",
                json!({ "ranges": ranges, "filters": [], "dimension": "path", "metric": metric }),
            )
            .await;
        response.assert_status_success();
        let body: serde_json::Value = response.json();
        let rows = body["reports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|report| {
                report
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| json!([row["dimensionValue"], row["value"]]))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(json!(rows), expected, "{metric}");
    }
    let response =
        client.post("/api/dashboard/project/public-project/stats", json!({ "ranges": ranges, "filters": [] })).await;
    response.assert_status_success();
    let body: serde_json::Value = response.json();
    assert_eq!(body["reports"][0]["bounceRate"], 1.0);
    assert_eq!(body["reports"][1]["avgTimeOnSite"], 200.0 / 3.0);
    assert!(body["reports"][2]["bounceRate"].is_null());
    assert!(body["reports"][2]["avgTimeOnSite"].is_null());

    let response = client
        .post("/api/dashboard/project/public-project/custom-events", json!({ "ranges": ranges, "filters": [] }))
        .await;
    response.assert_status_success();
    let body: serde_json::Value = response.json();
    assert_eq!(body["reports"][0]["rows"].as_array().unwrap().len(), 2);
    assert_eq!(body["reports"][0]["rows"][0]["completions"], 3);
    assert_eq!(body["reports"][0]["rows"][1]["completions"], 1);
    assert_eq!(body["reports"][0]["truncated"], true);
    assert_eq!(body["reports"][1]["rows"][0]["completions"], 2);
    assert_eq!(body["reports"][1]["rows"][1]["completions"], 1);
    assert_eq!(body["reports"][1]["truncated"], false);
    assert_eq!(body["reports"][2]["rows"].as_array().unwrap().len(), 2);
    assert!(body["reports"][2]["rows"][0]["completions"].is_null());
    assert!(body["reports"][2]["rows"][1]["completions"].is_null());
    assert_eq!(body["reports"][2]["truncated"], false);
    Ok(())
}

#[tokio::test]
async fn report_ranges_are_validated() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.seed_database(0)?;
    let valid = json!({ "start": "2026-01-01T00:00:00Z", "end": "2026-01-02T00:00:00Z" });
    for endpoint in ["stats", "graph", "dimension", "custom-events"] {
        for ranges in [
            json!([]),
            json!(vec![valid.clone(); 5]),
            json!([valid, { "start": "2026-01-02T00:00:00Z", "end": "2026-01-01T00:00:00Z" }]),
            json!([valid, { "start": "2000-01-01T00:00:00Z", "end": "2026-01-01T00:00:00Z" }]),
        ] {
            client.post(
                &format!("/api/dashboard/project/public-project/{endpoint}"),
                json!({ "ranges": ranges, "filters": [], "metric": "views", "dimension": "path", "interval": "day" }),
            ).await.assert_status_bad_request();
        }
    }
    // Every range is subject to the graph's datapoint limit.
    client
        .post(
            "/api/dashboard/project/public-project/graph",
            json!({
                "ranges": [valid, { "start": "2025-01-01T00:00:00Z", "end": "2026-01-01T00:00:00Z" }],
                "filters": [], "metric": "views", "interval": "hour",
            }),
        )
        .await
        .assert_status_bad_request();
    Ok(())
}

#[tokio::test]
async fn report_date_overflow_is_rejected() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.projects.create(
        &liwan::app::models::Project {
            id: "public".into(),
            display_name: "Public".into(),
            visibility: ProjectVisibility::Public,
            secret: None,
        },
        &[],
    )?;

    for (start, end, interval, timezone) in [
        ("+262142-12-31T23:00:00Z", "+262142-12-31T23:30:00Z", "hour", "UTC"),
        ("-262143-01-01T00:00:00Z", "-262143-01-02T00:00:00Z", "day", "America/New_York"),
        ("+262142-12-31T23:00:00Z", "+262142-12-31T23:30:00Z", "day", "Asia/Tokyo"),
    ] {
        client
            .post(
                "/api/dashboard/project/public/graph",
                json!({
                    "ranges": [{ "start": start, "end": end }],
                    "filters": [], "metric": "views", "interval": interval, "timezone": timezone,
                }),
            )
            .await
            .assert_status_bad_request();
    }
    Ok(())
}

#[tokio::test]
async fn test_dashboard() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let client = common::TestClient::new(app.clone(), tx);

    app.seed_database(100)?;

    let project_id = "public-project";
    let api_prefix = format!("/api/dashboard/project/{project_id}");
    let stats_path = format!("{api_prefix}/stats");
    let graph_path = format!("{api_prefix}/graph");
    let dimension_path = format!("{api_prefix}/dimension");

    let start_date = (Utc::now() - Duration::days(365)).to_rfc3339();
    let end_date = Utc::now().to_rfc3339();
    let ranges = json!([{ "start": start_date, "end": end_date }]);

    let stats_requests = [
        json!({"ranges": ranges,"filters":[]}),
        json!({"ranges": ranges,"filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"}]}),
    ];

    let graph_requests = [
        json!({"ranges": ranges,"metric":"views","interval":"day","timezone":"UTC","filters":[]}),
        json!({"ranges": ranges,"metric":"views","interval":"day","timezone":"UTC","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"}]}),
    ];

    let dimensions_requests = [
        json!({"dimension":"country","filters":[],"metric":"views","ranges": ranges}),
        json!({"dimension":"url","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","ranges": ranges}),
        json!({"dimension":"city","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","ranges": ranges}),
        json!({"dimension":"browser","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","ranges": ranges}),
        json!({"dimension":"screen_width","filters":[],"metric":"views","ranges": ranges}),
        json!({"dimension":"url","filters":[{"dimension":"screen_width","filterType":"equal","value":"xs"}],"metric":"views","ranges": ranges}),
    ];

    for request in stats_requests.iter() {
        let res = client.post(&stats_path, request.clone()).await;
        res.assert_status_success();
    }

    for request in graph_requests.iter() {
        let res = client.post(&graph_path, request.clone()).await;
        res.assert_status_success();
    }

    for request in dimensions_requests.iter() {
        let res = client.post(&dimension_path, request.clone()).await;
        res.assert_status_success();
    }

    Ok(())
}
