mod api;
mod app;
mod config;
mod reports;
mod utils;

use app::{App, Event};
use argh::FromArgs;
use config::Config;
use eyre::Result;

#[derive(FromArgs)]
/// liwan - lightweight web analytics
struct Args {
    #[argh(option)]
    /// path to the configuration file
    config: Option<String>,

    #[argh(subcommand)]
    cmd: Option<Command>,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    HashPassword(HashPassword),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "hash-password")]
/// hash a password (Usage: liwan hash-password <password>)
struct HashPassword {
    #[argh(positional)]
    /// the password to hash
    password: String,
}

/// liwan --config <config_file>
/// liwan
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();
    if let Some(Command::HashPassword(cmd)) = args.cmd {
        println!("Password hash: {}", utils::hash::hash_password(&cmd.password)?);
        return Ok(());
    }

    let config_path = args.config.as_deref().unwrap_or("liwan.config.toml");
    let config = Config::from_file(std::path::Path::new(config_path))?;

    let (s, r) = crossbeam::channel::unbounded::<Event>();
    let app = App::new(config)?;

    tokio::select! {
        res = api::start_webserver(app.clone(), s) => res,
        res = tokio::task::spawn_blocking(move || app.clone().process_events(r)) => res?
    }
}
