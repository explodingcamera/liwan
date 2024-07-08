mod api;
mod app;
mod cli;
mod config;
mod utils;

use app::{models::Event, App};
use config::Config;
use eyre::Result;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args = cli::args();
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
