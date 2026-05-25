use crate::{
    app::{Liwan, models::UserRole},
    config::{Config, DEFAULT_CONFIG, GeoIpConfig},
};
use anyhow::Result;
use argh::FromArgs;

#[derive(FromArgs)]
/// liwan - lightweight web analytics
pub struct Args {
    #[argh(option)]
    /// path to the configuration file
    pub config: Option<String>,

    #[argh(option, default = "tracing::Level::INFO")]
    /// set the log level (default: INFO)
    pub log_level: tracing::Level,

    #[argh(subcommand)]
    pub cmd: Option<Command>,
}

pub fn args() -> Args {
    argh::from_env()
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Command {
    GenerateConfig(GenConfig),
    UpdatePassword(UpdatePassword),
    AddUser(AddUser),
    Users(ListUsers),
    Prune(Prune),
    #[cfg(any(debug_assertions, test, feature = "__dev"))]
    SeedDatabase(SeedDatabase),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "prune")]
/// Prune collection data according to current UI-managed settings
pub struct Prune {
    #[argh(switch)]
    /// show what would be pruned without changing data
    dry_run: bool,
}

#[cfg(any(debug_assertions, test, feature = "__dev"))]
#[derive(FromArgs)]
#[argh(subcommand, name = "seed-database")]
/// Seed the database with some test data
pub struct SeedDatabase {}

#[derive(FromArgs)]
#[argh(subcommand, name = "generate-config")]
/// Save a default configuration file to `liwan.config.toml`
pub struct GenConfig {
    #[argh(option, short = 'o')]
    /// the path to write the configuration file to
    output: Option<String>,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "update-password")]
/// Update a user's password
pub struct UpdatePassword {
    #[argh(positional)]
    /// the username of the user to update
    username: String,

    #[argh(positional)]
    /// the new password
    password: String,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "users")]
/// List all registered users
pub struct ListUsers {}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-user")]
/// Create a new user
pub struct AddUser {
    #[argh(positional)]
    /// the username of the new user
    username: String,

    #[argh(positional)]
    /// the password of the new user
    password: String,

    #[argh(option, default = "false")]
    /// assign the user the admin role
    admin: bool,
}

pub fn handle_command(mut config: Config, cmd: Command) -> Result<()> {
    config.geoip = GeoIpConfig::default(); // disable GeoIP in CLI commands

    match cmd {
        Command::UpdatePassword(update) => {
            let app = Liwan::try_new(config)?;
            app.users.update_password(&update.username, &update.password)?;
            println!("Password updated for user {}", update.username);
        }
        Command::Users(_) => {
            let app = Liwan::try_new(config)?;
            let users = app.users.all()?;
            if users.is_empty() {
                println!("No users found");
                println!("Use `liwan add-user` to create a new user");
                return Ok(());
            }

            println!("Users:");
            for user in users {
                println!(" - {} ({})", user.username, user.role);
            }
        }
        Command::AddUser(add) => {
            let app = Liwan::try_new(config)?;
            app.users.create(
                &add.username,
                &add.password,
                if add.admin { UserRole::Admin } else { UserRole::User },
                &[],
            )?;

            println!("User {} created", add.username);
        }
        Command::GenerateConfig(GenConfig { output }) => {
            let output = output.unwrap_or_else(|| "liwan.config.toml".to_string());
            if std::path::Path::new(&output).exists() {
                println!("Configuration file already exists");
                return Ok(());
            }

            std::fs::write(&output, DEFAULT_CONFIG)?;
            println!("Configuration file written to liwan.config.toml");
        }
        Command::Prune(prune) => {
            let app = Liwan::try_new(config)?;
            let mut totals = crate::app::PruneStats::default();
            for entity in app.entities.all()? {
                let settings = app.settings.resolved_for_entity(&entity.id);
                let stats = app.events.prune_entity(&entity.id, &settings, prune.dry_run)?;
                println!(
                    "{}: total={}, delete={}, clear_utm={}, clear_geo={}, clear_sessions={}",
                    entity.id,
                    stats.total_events,
                    stats.deleted_events,
                    stats.cleared_utm_events,
                    stats.cleared_geo_events,
                    stats.cleared_session_events
                );
                totals.total_events += stats.total_events;
                totals.deleted_events += stats.deleted_events;
                totals.cleared_utm_events += stats.cleared_utm_events;
                totals.cleared_geo_events += stats.cleared_geo_events;
                totals.cleared_session_events += stats.cleared_session_events;
            }
            println!(
                "total: total={}, delete={}, clear_utm={}, clear_geo={}, clear_sessions={}",
                totals.total_events,
                totals.deleted_events,
                totals.cleared_utm_events,
                totals.cleared_geo_events,
                totals.cleared_session_events
            );
            if prune.dry_run {
                println!("Dry run only. Re-run without --dry-run to apply changes.");
            }
        }
        #[cfg(any(debug_assertions, test, feature = "__dev"))]
        Command::SeedDatabase(_) => {
            let app = Liwan::try_new(config)?;
            app.seed_database(10_000_000)?;
            println!("Database seeded with test data");
        }
    }

    Ok(())
}
