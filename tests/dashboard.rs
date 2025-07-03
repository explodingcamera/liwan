mod common;
use chrono::{Duration, Utc};
use eyre::{Result, bail};
use serde_json::json;

#[tokio::test]
async fn test_dashboard() -> Result<()> {
    let app = common::app();
    let (tx, _rx) = common::events();
    let router = common::router(app.clone(), tx);
    let client = poem::test::TestClient::new(router);

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
        json!({"range":{"start": start_date ,"end": end_date},"metric":"views","dataPoints":395,"filters":[]}),
        json!({"range":{"start": start_date ,"end": end_date},"metric":"views","dataPoints":30,"filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"}]}),
    ];

    let dimensions_requests = [
        json!({"dimension":"country","filters":[],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"url","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"city","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","range":{"start": start_date ,"end": end_date}}),
        json!({"dimension":"browser","filters":[{"dimension":"fqdn","filterType":"equal","value":"example.org"},{"dimension":"url","filterType":"equal","value":"example.org/contact"},{"dimension":"referrer","filterType":"equal","value":"liwan.dev"},{"dimension":"country","filterType":"equal","value":"AU"},{"dimension":"city","filterType":"equal","value":"Sydney"},{"dimension":"platform","filterType":"equal","value":"iOS"},{"dimension":"browser","filterType":"equal","value":"Safari"},{"dimension":"mobile","filterType":"is_true"}],"metric":"views","range":{"start": start_date ,"end": end_date}}),
    ];

    for request in stats_requests.iter() {
        let res = client.post(stats_path.clone()).body_json(request).send().await;
        let status = res.0.status();
        if !status.is_success() {
            bail!("Failed to get stats: status: {}, body: {:?}", status, res.0.into_body().into_string().await);
        }
    }

    for request in graph_requests.iter() {
        let res = client.post(graph_path.clone()).body_json(request).send().await;
        let status = res.0.status();
        if !status.is_success() {
            bail!("Failed to get graph: status: {}, body: {:?}", status, res.0.into_body().into_string().await);
        }
    }

    for request in dimensions_requests.iter() {
        let res = client.post(dimension_path.clone()).body_json(request).send().await;
        let status = res.0.status();
        if !status.is_success() {
            bail!(
                "Failed to get dimension report: status: {}, body: {:?}",
                status,
                res.0.into_body().into_string().await
            );
        }
    }

    Ok(())
}
