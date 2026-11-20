use crate::{
    app::{Liwan, models::UserRole},
    config::{Config, DEFAULT_CONFIG, GeoIpConfig},
    utils::duckdb::{LATEST_STORAGE_FORMAT, requires_newer_duckdb, storage_format},
};
use anyhow::{Context, Result, bail};
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
    UpgradeDuckDb(UpgradeDuckDb),
    #[cfg(debug_assertions)]
    Dev(Dev),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "prune")]
/// Prune collection data according to current UI-managed settings
pub struct Prune {
    #[argh(switch)]
    /// show what would be pruned without changing data
    dry_run: bool,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "upgrade-duckdb")]
/// Upgrade a DuckDB file if it uses an older storage format, saving the original as .old
pub struct UpgradeDuckDb {
    #[argh(positional)]
    /// path to the database (defaults to the configured event database; stop Liwan first)
    path: Option<std::path::PathBuf>,
}

#[derive(FromArgs)]
#[cfg(debug_assertions)]
#[argh(subcommand, name = "dev")]
/// Debug-only development commands
pub struct Dev {
    #[argh(subcommand)]
    cmd: DevCommand,
}

#[derive(FromArgs)]
#[cfg(debug_assertions)]
#[argh(subcommand)]
pub enum DevCommand {
    Seed(SeedDatabase),
    GenerateOpenApi(GenerateOpenApi),
    ResetDb(ResetDb),
}

#[derive(FromArgs)]
#[cfg(debug_assertions)]
#[argh(subcommand, name = "seed")]
/// Seed the database with some test data
pub struct SeedDatabase {}

#[derive(FromArgs)]
#[cfg(debug_assertions)]
#[argh(subcommand, name = "generate-openapi")]
/// Regenerate the TypeScript OpenAPI definition
pub struct GenerateOpenApi {}

#[derive(FromArgs)]
#[cfg(debug_assertions)]
#[argh(subcommand, name = "reset-db")]
/// Delete local app and events database files
pub struct ResetDb {}

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
            if !app.users.update_password(&update.username, &update.password, None, None)? {
                anyhow::bail!("Failed to update password");
            }
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
            app.users.create(&add.username, &add.password, if add.admin { UserRole::Admin } else { UserRole::User })?;

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
        Command::UpgradeDuckDb(upgrade) => {
            let path =
                upgrade.path.unwrap_or_else(|| std::path::Path::new(&config.data_dir).join("liwan-events.duckdb"));
            if let Some(backup) = upgrade_db(&path)? {
                println!("Upgraded {} (old file saved to {})", path.display(), backup.display());
            }
        }
        #[cfg(debug_assertions)]
        Command::Dev(dev) => match dev.cmd {
            DevCommand::Seed(_) => {
                let app = Liwan::try_new(config)?;
                app.seed_database(10_000_000)?;
                println!("Database seeded with test data");
            }
            DevCommand::GenerateOpenApi(_) => {
                let app = Liwan::try_new(config)?;
                let (events, _) = tokio::sync::mpsc::channel(1);
                let (exits, _) = tokio::sync::mpsc::channel(1);
                let (_, spec) = crate::web::router(app, crate::web::EventQueues { events, exits })?;
                crate::web::save_spec(spec)?;
                println!("OpenAPI definition generated");
            }
            DevCommand::ResetDb(_) => {
                let dir = std::path::Path::new(&config.data_dir);
                for file in [
                    "liwan-app.sqlite",
                    "liwan-app.sqlite-shm",
                    "liwan-app.sqlite-wal",
                    "liwan-events.duckdb",
                    "liwan-events.duckdb.wal",
                ] {
                    let path = dir.join(file);
                    if path.exists() {
                        std::fs::remove_file(&path)?;
                        println!("Deleted {}", path.display());
                    }
                }
            }
        },
    }

    Ok(())
}

fn upgrade_db(path: &std::path::Path) -> Result<Option<std::path::PathBuf>> {
    let metadata = path.symlink_metadata().with_context(|| format!("Failed to inspect {}", path.display()))?;
    if !metadata.is_file() {
        bail!("Not a regular DuckDB file: {}", path.display());
    }
    let wal = std::path::PathBuf::from(format!("{}.wal", path.display()));
    if wal.symlink_metadata().is_ok() {
        bail!("Database has a WAL file at {}; stop Liwan and checkpoint it before upgrading", wal.display());
    }
    let backup = std::path::PathBuf::from(format!("{}.old", path.display()));
    let staging = std::path::PathBuf::from(format!("{}.upgrade", path.display()));
    let upgraded = staging.join("copy.duckdb");
    let source_sql = path.to_str().context("Source path is not valid UTF-8")?.replace('\'', "''");
    {
        let conn = duckdb::Connection::open_in_memory()?;
        conn.execute_batch(&format!("ATTACH '{source_sql}' AS liwan_source (READ_ONLY)"))
            .map_err(|err| {
                let cause = err.to_string();
                let message = if requires_newer_duckdb(&cause) {
                    "This file uses a newer DuckDB storage format than this Liwan build supports. Upgrade Liwan instead. No files were changed"
                } else if cause.contains("Could not set lock on file") {
                    "Failed to open database. Stop Liwan before upgrading"
                } else {
                    "Failed to open database. No files were changed"
                };
                anyhow::Error::new(err).context(message)
            })?;
        let source_version =
            storage_format(&conn, Some("liwan_source")).context("Failed to read DuckDB storage format")?;
        if source_version == LATEST_STORAGE_FORMAT {
            println!("{} already uses the latest DuckDB storage format ({source_version})", path.display());
            return Ok(None);
        }
        if backup.symlink_metadata().is_ok() {
            bail!("Destination already exists: {}", backup.display());
        }
        let builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        let builder = {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = builder;
            builder.mode(0o700);
            builder
        };
        builder
            .create(&staging)
            .with_context(|| format!("Failed to create upgrade directory {}", staging.display()))?;

        println!("Upgrading {} from DuckDB storage format {source_version} to {LATEST_STORAGE_FORMAT}", path.display());
        conn.execute_batch(&format!(
            "SET temp_directory = '{source_sql}.upgrade/spill';
             ATTACH '{source_sql}.upgrade/copy.duckdb' AS liwan_upgraded (STORAGE_VERSION '{LATEST_STORAGE_FORMAT}')"
        ))
        .context("Failed to create upgraded DuckDB database")?;
        conn.execute_batch("COPY FROM DATABASE liwan_source TO liwan_upgraded; CHECKPOINT liwan_upgraded")
            .with_context(|| format!("Failed to copy database; {} may contain a partial copy", upgraded.display()))?;
    }

    std::fs::set_permissions(&upgraded, metadata.permissions()).context("Failed to preserve database permissions")?;
    std::fs::rename(path, &backup).context("Failed to save original database as .old")?;
    if let Err(err) = std::fs::rename(&upgraded, path) {
        std::fs::rename(&backup, path).context("Failed to install upgraded database and restore original")?;
        return Err(err).context("Failed to install upgraded database; original was restored");
    }
    if let Err(err) = std::fs::remove_dir(&staging) {
        tracing::warn!(%err, path = %staging.display(), "Failed to remove upgrade directory");
    }
    Ok(Some(backup))
}

#[cfg(test)]
mod tests {
    use super::{Args, Command, UpgradeDuckDb, handle_command, upgrade_db};
    use crate::utils::duckdb::{LATEST_STORAGE_FORMAT, storage_format};
    use argh::FromArgs;
    use duckdb::Connection;

    #[test]
    fn upgrade_command_accepts_optional_path() {
        let args = Args::from_args(&["liwan"], &["upgrade-duckdb"]).unwrap();
        assert!(matches!(args.cmd, Some(Command::UpgradeDuckDb(upgrade)) if upgrade.path.is_none()));

        let args = Args::from_args(&["liwan"], &["upgrade-duckdb", "other.duckdb"]).unwrap();
        assert!(
            matches!(args.cmd, Some(Command::UpgradeDuckDb(upgrade)) if upgrade.path == Some("other.duckdb".into()))
        );
    }

    #[test]
    fn upgrade_command_uses_configured_event_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("liwan-events.duckdb");
        let flags = duckdb::Config::default().with("storage_compatibility_version", "v1.5.0").unwrap();
        Connection::open_with_flags(&path, flags)
            .unwrap()
            .execute_batch("CREATE TABLE events (id INTEGER); CHECKPOINT")
            .unwrap();
        let mut config = crate::config::Config::default();
        config.data_dir = dir.path().to_str().unwrap().into();

        handle_command(config, Command::UpgradeDuckDb(UpgradeDuckDb { path: None })).unwrap();

        assert!(path.exists());
        assert!(dir.path().join("liwan-events.duckdb.old").exists());
    }

    #[test]
    fn upgrades_database_and_keeps_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        let flags = duckdb::Config::default().with("storage_compatibility_version", "v1.5.0").unwrap();
        {
            let conn = Connection::open_with_flags(&path, flags).unwrap();
            conn.execute_batch(
                "CREATE TABLE events (id INTEGER PRIMARY KEY, properties MAP(VARCHAR, VARCHAR));
                 INSERT INTO events VALUES (42, MAP(['plan'], ['pro']));
                 CHECKPOINT",
            )
            .unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        let backup = upgrade_db(&path).unwrap().unwrap();
        assert_eq!(backup, dir.path().join("events.duckdb.old"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        }
        let original = Connection::open(&backup).unwrap();
        let upgraded = Connection::open(&path).unwrap();
        for conn in [&original, &upgraded] {
            let id: i32 = conn.query_row("SELECT id FROM events", [], |row| row.get(0)).unwrap();
            assert_eq!(id, 42);
            let plan: String = conn
                .query_row("SELECT map_extract_value(properties, 'plan') FROM events", [], |row| row.get(0))
                .unwrap();
            assert_eq!(plan, "pro");
        }
        assert_eq!(storage_format(&original, None).unwrap(), "v1.5.0");
        assert_eq!(storage_format(&upgraded, None).unwrap(), LATEST_STORAGE_FORMAT);
        drop(upgraded);
        assert!(upgrade_db(&path).unwrap().is_none());
        assert!(!dir.path().join("events.duckdb.upgrade").exists());
    }

    #[test]
    fn current_database_is_not_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        let flags = duckdb::Config::default().with("storage_compatibility_version", "latest").unwrap();
        Connection::open_with_flags(&path, flags)
            .unwrap()
            .execute_batch("CREATE TABLE events (id INTEGER); CHECKPOINT")
            .unwrap();
        let original = std::fs::read(&path).unwrap();

        assert!(upgrade_db(&path).unwrap().is_none());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(!dir.path().join("events.duckdb.old").exists());
        assert!(!dir.path().join("events.duckdb.upgrade").exists());
    }

    #[test]
    fn existing_backup_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        let flags = duckdb::Config::default().with("storage_compatibility_version", "v1.5.0").unwrap();
        Connection::open_with_flags(&path, flags)
            .unwrap()
            .execute_batch("CREATE TABLE events (id INTEGER); CHECKPOINT")
            .unwrap();
        let backup = dir.path().join("events.duckdb.old");
        std::fs::write(&backup, "keep me").unwrap();

        assert!(upgrade_db(&path).unwrap_err().to_string().contains("Destination already exists"));
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "keep me");
        assert!(path.exists());
        assert!(!dir.path().join("events.duckdb.upgrade").exists());
    }

    #[test]
    fn refuses_invalid_database_without_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        let original = b"not a DuckDB database";
        std::fs::write(&path, original).unwrap();

        let err = upgrade_db(&path).unwrap_err();
        assert!(err.to_string().contains("No files were changed"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(dir.path().read_dir().unwrap().count(), 1);
    }

    #[test]
    fn refuses_database_with_wal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.duckdb");
        Connection::open(&path).unwrap().execute_batch("CREATE TABLE events (id INTEGER)").unwrap();
        let wal = dir.path().join("events.duckdb.wal");
        std::fs::write(&wal, "pending").unwrap();

        assert!(upgrade_db(&path).unwrap_err().to_string().contains("WAL file"));
        assert!(path.exists());
        assert!(!dir.path().join("events.duckdb.old").exists());
    }
}
