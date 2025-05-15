#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

use eyre::Result;

use liwan::app::{Liwan, models::Event};
use liwan::{cli, config::Config, web};
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider().install_default().expect("failed to install crypto provider");

    let args = cli::args();
    setup_logger(args.log_level)?;

    let config = Config::load(args.config)?;
    let (s, r) = crossbeam_channel::unbounded::<Event>();

    if let Some(cmd) = args.cmd {
        return cli::handle_command(config, cmd);
    }

    let app = Liwan::try_new(config)?;
    let app_copy = app.clone();
    app.run_background_tasks();

    tokio::select! {
        _ = tokio::signal::ctrl_c() => app_copy.shutdown(),
        res = web::start_webserver(app.clone(), s) => res,
        res = tokio::task::spawn_blocking(move || app.clone().events.process(r)) => res?
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
