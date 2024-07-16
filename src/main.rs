pub(crate) mod api;
pub(crate) mod app;
pub(crate) mod cli;
pub(crate) mod config;
pub(crate) mod utils;

use app::{models::Event, App};
use config::Config;
use eyre::Result;
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = cli::args();

    // external crates should use WARN
    let filter = EnvFilter::from_default_env()
        .add_directive(format!("{}={}", env!("CARGO_PKG_NAME"), args.log_level).parse()?)
        .add_directive(Level::WARN.into());
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    let config = Config::load(args.config)?;
    let (s, r) = crossbeam::channel::unbounded::<Event>();

    let app = App::try_new(config)?;

    if let Some(cmd) = args.cmd {
        return cli::handle_command(app, cmd);
    }

    tokio::select! {
        res = api::start_webserver(app.clone(), s) => res,
        res = tokio::task::spawn_blocking(move || app.clone().process_events(r)) => res?
    }
}
