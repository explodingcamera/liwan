use crate::{
    app::{models::UserRole, App},
    config::DEFAULT_CONFIG,
};
use argh::FromArgs;
use colored::Colorize;
use eyre::Result;

#[derive(FromArgs)]
/// liwan - lightweight web analytics
pub(crate) struct Args {
    #[argh(option)]
    /// path to the configuration file
    pub(crate) config: Option<String>,

    #[argh(subcommand)]
    pub(crate) cmd: Option<Command>,
}

pub(crate) fn args() -> Args {
    argh::from_env()
}

#[derive(FromArgs)]
#[argh(subcommand)]
pub(crate) enum Command {
    GenerateConfig(GenConfig),
    UpdatePassword(UpdatePassword),
    AddUser(AddUser),
    Users(ListUsers),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "generate-config")]
/// Save a default configuration file to `liwan.config.toml`
pub(crate) struct GenConfig {}

#[derive(FromArgs)]
#[argh(subcommand, name = "update-password")]
/// Update a user's password
pub(crate) struct UpdatePassword {
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
pub(crate) struct ListUsers {}

#[derive(FromArgs)]
#[argh(subcommand, name = "add-user")]
/// Create a new user
pub(crate) struct AddUser {
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

pub(crate) fn handle_command(app: App, cmd: Command) -> Result<()> {
    match cmd {
        Command::UpdatePassword(update) => {
            app.user_update_password(&update.username, &update.password)?;
            println!("Password updated for user {}", update.username);
        }
        Command::Users(_) => {
            let users = app.users()?;
            if users.is_empty() {
                println!("{}", "No users found".bold());
                println!("Use `liwan add-user` to create a new user");
                return Ok(());
            }

            println!("{}", "Users:".bold());
            for user in users {
                println!(" - {} ({:?})", user.username.underline(), user.role);
            }
        }
        Command::AddUser(add) => {
            app.user_create(
                &add.username,
                &add.password,
                match add.admin {
                    true => UserRole::Admin,
                    false => UserRole::User,
                },
                vec![],
            )?;

            println!("User {} created", add.username);
        }
        Command::GenerateConfig(_) => {
            if std::path::Path::new("liwan.config.toml").exists() {
                println!("Configuration file already exists");
                return Ok(());
            }

            std::fs::write("liwan.config.toml", DEFAULT_CONFIG)?;
            println!("Configuration file written to liwan.config.toml");
        }
    }

    Ok(())
}
