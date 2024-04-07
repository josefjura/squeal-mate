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
use color_eyre::eyre;
use components::help::Help;
use components::script_status::ScriptStatus;
use components::scroll_list::ScrollList;
use components::status::Status;
use config::{get_config_dir, get_data_dir, Settings};
use crossterm::{execute, style::Print};
use db::Database;
use error::ArgumentsError;
use ratatui::style::Stylize;
use repository::{Repository, RepositoryError};
use std::io::{self, stdout};
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
            let status = Status::new();
            let script_status = ScriptStatus::new();
            let scroll_list = ScrollList::new(connection.clone(), path);

            let mut app = App::new(
                vec![
                    Screen::new(
                        Mode::FileChooser,
                        vec![Box::new(list), Box::new(status), Box::new(Help::new())],
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

fn draw_help(stdout: &mut io::Stdout) -> eyre::Result<()> {
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

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut stdout = io::stdout();

    let config = Settings::new().expect("Error while loading config!");

    let args = AeqArgs::parse();

    match args.command {
        Some(Command::Config) => {
            draw_help(&mut stdout)?;
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
    }

    Ok(())
}
