mod action;
mod app;
mod batch_parser;
mod cli;
mod components;
mod config;
mod db;
mod entries;
mod error;
mod repository;
mod screen;
mod tui;
mod utils;

use crate::screen::{Mode, Screen};

use crate::app::App;
use crate::components::list::List;
use clap::Parser;
use cli::{AeqArgs, Command};
use cliclack::{confirm, input, intro, outro};

use color_eyre::eyre;
use components::help::Help;
use components::script_status::ScriptStatus;
use components::scroll_list::ScrollList;
use config::{get_config_dir, get_data_dir, Settings};
use crossterm::style::Stylize;
use crossterm::{execute, style::Print};
use db::Database;
use error::ArgumentsError;
use repository::{Repository, RepositoryError};
use std::env;
use std::io::{self, stdout};
use std::path::Path;
use std::{io::Write, path::PathBuf, str::FromStr};
use utils::{initialize_logging, initialize_panic_handler};

async fn start_tui(config: Settings, connection: Database) -> eyre::Result<()> {
    initialize_logging()?;

    initialize_panic_handler()?;

    let path: PathBuf = if let Some(ref content) = config.repository.path {
        PathBuf::from(content)
    } else {
        PathBuf::from_str("./").expect("Can't open current directory")
    };

    let repository = Repository::new(path.clone());

    match repository {
        Ok(repository) => {
            let list = List::new(repository);
            let script_status = ScriptStatus::new();
            let scroll_list = ScrollList::new(connection.clone(), path);

            let mut app = App::new(
                vec![
                    Screen::new(
                        Mode::FileChooser,
                        vec![Box::new(list), Box::new(Help::new())],
                    ),
                    Screen::new(
                        Mode::ScriptRunner,
                        vec![
                            Box::new(scroll_list),
                            Box::new(script_status),
                            Box::new(Help::new()),
                        ],
                    ),
                ],
                config,
            );

            app.run().await?;
            execute!(
                stdout(),
                Print("🦀 Thank you for using AEQ-CAC 🦀\n".yellow())
            )?;
            Ok(())
        }
        Err(RepositoryError::DoesNotExist) => {
            log::error!("Repository does not exist");
            Ok(())
        }
        Err(RepositoryError::NotUTF8) => {
            log::error!("Repository configuration is not UTF8");
            Ok(())
        }
        Err(RepositoryError::IOError(e)) => {
            log::error!("Internal IO error: {}", e);
            Ok(())
        }
    }
}

fn draw_config(stdout: &mut io::Stdout) -> eyre::Result<()> {
    let config_path = get_config_dir();
    let data_path = get_data_dir();
    let config_path_str = config_path.to_str().expect("Unknown host system").white();
    let data_path_str = data_path.to_str().expect("Unknown host system").white();
    let version = env!("CARGO_PKG_VERSION").white();
    let version_msg = format!("Version: {}\n", version);
    let config_msg = format!("Config src: {}\n", config_path_str);
    let data_msg = format!("Logs dir: {}\n", data_path_str);
    execute!(
        stdout,
        Print("🦀 Aequitas Command And Control Console 🦀\n".yellow()),
        Print("\n"),
        Print(version_msg),
        Print("Edition: "),
        Print("Ultimate\n\n".white()),
        Print(config_msg),
        Print(data_msg)
    )?;

    stdout.flush()?;
    Ok(())
}

fn init_config() -> eyre::Result<()> {
    intro("Aequitas Command And Control Console")?;

    let mut settings = Settings {
        database: config::Database {
            integrated: None,
            username: None,
            password: None,
            server: None,
            port: None,
            name: None,
        },
        repository: config::Repository { path: None },
    };

    let current = env::current_dir()?;
    let current_string = current.to_str().expect("Unknown host system").to_string();

    let repository: String = input("Where are the migrations stored?")
        .default_input(&current_string)
        .validate(|input: &String| {
            let exists = Path::new(input).exists();
            if exists {
                Ok(())
            } else {
                Err("Enter existing directory.")
            }
        })
        .interact()?;
    settings.repository.path = Some(repository);

    let database: String = input("Database url")
        .default_input("localhost")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Please enter a username")
            } else {
                Ok(())
            }
        })
        .interact()?;
    settings.database.server = Some(database);

    let port: String = input("Database port")
        .default_input("1433")
        .validate(|input: &String| match input.parse::<u16>() {
            Ok(_) => Ok(()),
            Err(_) => Err("Port must be a number"),
        })
        .interact()?;
    settings.database.port = Some(port.parse::<u16>().unwrap());

    let integrated: bool = confirm("Do you want to use integrated security to connect to database? (e.g. Windows Authentication)")
		.initial_value(true)
		.interact()?;
    settings.database.integrated = Some(integrated);

    if !integrated {
        let username: String = input("SQL user name")
            .validate(|input: &String| {
                if input.is_empty() {
                    Err("Username cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact()?;
        settings.database.username = Some(username);

        let store_password: bool = confirm(
            "Do you want to store the password in the configuration file? (Not recommended)",
        )
        .initial_value(false)
        .interact()?;

        if store_password {
            let password: String = input("SQL user password")
                .validate(|input: &String| {
                    if input.is_empty() {
                        Err("Password cannot be empty")
                    } else {
                        Ok(())
                    }
                })
                .interact()?;
            settings.database.password = Some(password);
        }
    }

    let db_name: String = input("Datababase name")
        .validate(|input: &String| {
            if input.is_empty() {
                Err("Database name cannot be empty")
            } else {
                Ok(())
            }
        })
        .interact()?;
    settings.database.name = Some(db_name);

    if let Some(ref path) = settings.repository.path {
        cliclack::log::info(format!("Repository path: {}", path))?;
    }

    if let Some(ref integrated) = settings.database.integrated {
        if *integrated {
            cliclack::log::info("Using integrated authentication")?;
        } else {
            if let Some(ref username) = settings.database.username {
                cliclack::log::info(format!("SQl user name: {}", username))?;
            }
            if let Some(ref password) = settings.database.password {
                cliclack::log::info(format!("SQl user password: {}", password))?;
            }
        }
    }

    if let Some(ref server) = settings.database.server {
        cliclack::log::info(format!("Database server: {}", server))?;
    }
    if let Some(ref port) = settings.database.port {
        cliclack::log::info(format!("Database port: {}", port))?;
    }
    if let Some(ref db_name) = settings.database.name {
        cliclack::log::info(format!("Database name: {}", db_name))?;
    }

    let can_save: bool = confirm(
        "Do you want to save the configuration? (If you choose no, the configuration will be lost)",
    )
    .initial_value(true)
    .interact()?;

    if !can_save {
        outro("Configuration not saved!")?;
        return Ok(());
    }

    let result = settings.save();

    match result {
        Ok(_) => {
            outro("Configuration saved!")?;
        }
        Err(config::SettingSaveError::SerializationError(e)) => {
            log::error!("Serialization error: {}", e);
            outro("Error while saving configuration!")?;
        }
        Err(config::SettingSaveError::WriteError(e)) => {
            log::error!("Write error: {}", e);
            outro("Error while saving configuration!")?;
        }
    }

    stdout().flush()?;

    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut stdout = io::stdout();

    let config = Settings::new().expect("Error while loading config!");

    let args = AeqArgs::parse();

    match args.command {
        Some(Command::Config) => {
            draw_config(&mut stdout)?;
        }
        Some(Command::Migrations) | None => {
            match args.connection.merge(&config) {
                Ok(conn) => start_tui(config, conn).await?,
                Err(ArgumentsError::MissingPassword) => {
                    println!("ERROR: Missing DB password");
                }
                Err(ArgumentsError::MissingUsername) => {
                    println!("ERROR: Missing DB username");
                }
                Err(ArgumentsError::MissingDBName) => {
                    println!("ERROR: Missing DB name");
                }
                Err(ArgumentsError::PortNotNumber) => {
                    println!("ERROR: Supplied port is not a valid number");
                }
            };
        }
        Some(Command::Initialize) => init_config()?,
    }

    Ok(())
}
