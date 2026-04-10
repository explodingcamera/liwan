#![forbid(unsafe_code)]
use anyhow::Result;

use liwan::app::{Liwan, models::Event};
use liwan::{cli, config::Config, web};
use tracing_subscriber::EnvFilter;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = cli::args();
    setup_logger(args.log_level)?;

    let config = Config::load(args.config, std::env::vars())?;
    let (s, r) = tokio::sync::mpsc::channel::<Event>(1024 * 10);

    if let Some(cmd) = args.cmd {
        return cli::handle_command(config, cmd);
    }

    let app = Liwan::try_new(config)?;
    let app_copy = app.clone();
    app.run_background_tasks();

    tokio::select! {
        biased;
        _ = liwan::utils::signals::shutdown() => app_copy.shutdown(),
        res = web::start_webserver(app.clone(), s) => res,
        res = app.events.process_events(r) => res,
    }
}

fn setup_logger(log_level: tracing::Level) -> Result<()> {
    // external crates should use WARN
    let filter = EnvFilter::from_default_env()
        .add_directive(format!("{}={}", env!("CARGO_PKG_NAME"), log_level).parse()?)
        .add_directive(tracing::Level::WARN.into());

    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    #[cfg(debug_assertions)]
    tracing::info!("Running in debug mode");
    Ok(())
}
