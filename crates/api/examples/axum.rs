use axum::{Router, routing::get};
use liwan_api::{Client, LiwanLayer, PathFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("LIWAN_API_KEY")?;
    let client = Client::builder("https://analytics.example.com", api_key, reqwest::Client::builder().build()?)?
        .build_tokio()?;

    let tracking = LiwanLayer::new(client.clone(), "docs")
        .origin("https://service.example.com")
        .paths(PathFilter::new().include("/").exclude("/health"));

    let _app: Router = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/docs", get(|| async { "documentation" }))
        .layer(tracking);

    // Run the router, then flush accepted events during graceful shutdown.
    client.shutdown().await?;
    Ok(())
}
