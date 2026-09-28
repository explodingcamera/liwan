mod common;
use anyhow::Result;
use chrono::{Duration, Utc};
use serde_json::json;

#[tokio::test]
async fn report_date_overflow_is_rejected() -> Result<()> {
    let app = common::app();
    let (queues, _receivers) = common::events();
    let client = common::TestClient::new(app.clone(), queues);
    app.projects.create(
        &liwan::app::models::Project {
            id: "public".into(),
            display_name: "Public".into(),
            public: true,
            unlisted: false,
            secret: None,
        },
        &[],
    )?;

    client
        .post(
            "/api/dashboard/project/public/stats",
            json!({
                "range": { "start": "-262143-01-02T00:00:00Z", "end": "-262143-01-04T00:00:00Z" },
                "filters": [],
            }),
        )
        .await
        .assert_status_bad_request();

    for (start, end, interval, timezone) in [
        ("+262142-12-31T23:00:00Z", "+262142-12-31T23:30:00Z", "hour", "UTC"),
        ("-262143-01-01T00:00:00Z", "-262143-01-02T00:00:00Z", "day", "America/New_York"),
        ("+262142-12-31T23:00:00Z", "+262142-12-31T23:30:00Z", "day", "Asia/Tokyo"),
    ] {
        client
            .post(
                "/api/dashboard/project/public/graph",
                json!({
                    "range": { "start": start, "end": end },
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

    let stats_requests = [
        json!({"range":{"start": start_date ,"end": end_date},"filters":[]}),
        json!({"range":{"start": start_date ,"end": end_date},"filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"}]}),
    ];

    let graph_requests = [
        json!({"range":{"start": start_date ,"end": end_date},"metric":"views","interval":"day","timezone":"UTC","filters":[]}),
        json!({"range":{"start": start_date ,"end": end_date},"metric":"views","interval":"day","timezone":"UTC","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"}]}),
    ];

    let dimensions_requests = [
        json!({"dimension":"country","filters":[],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"url","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"city","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"browser","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"screen_width","filters":[],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"url","filters":[{"dimension":"screen_width","filterType":"equal","value":"xs"}],"metric":"views","range":{"start": start_date ,"end": end_date}}),
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
