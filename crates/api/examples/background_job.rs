use liwan_api::{Client, Event};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("LIWAN_API_KEY")?;
    let analytics = Client::builder("https://analytics.example.com", api_key, reqwest::Client::builder().build()?)?
        .batch_size(100)
        .build_tokio()?;

    analytics.event("worker", Event::new("job_completed", "https://worker.example.com/jobs/import"))?;
    analytics.shutdown().await?;
    Ok(())
}
