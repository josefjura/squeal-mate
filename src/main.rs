mod action;
mod app;
mod cli;
mod components;
mod config;
mod db;
mod entries;
mod error;
mod repository;
mod tui;
mod utils;

use crate::app::{App, Mode};
use crate::components::list::List;
use app::Screen;
use clap::Parser;
use cli::{AeqArgs, Command};
use color_eyre::eyre;
use components::scroll_list::ScrollList;
use components::status::Status;
use config::{get_config_dir, read_config};
use crossterm::{execute, style::Print};
use db::Database;
use error::ArgumentsError;
use ratatui::style::Stylize;
use repository::{Repository, RepositoryError};
use std::io::{self};
use std::{collections::HashMap, io::Write, path::PathBuf, str::FromStr};
use utils::{initialize_logging, initialize_panic_handler};

async fn start_tui(config: HashMap<String, String>, connection: Database) -> eyre::Result<()> {
    initialize_logging()?;

    initialize_panic_handler()?;

    let path: PathBuf = if let Some(content) = config.get("path") {
        PathBuf::from(content)
    } else {
        PathBuf::from_str("./").expect("Can't open current directory")
    };

    let repository = Repository::new(path);

    match repository {
        Ok(repository) => {
            let list = List::new(repository, connection.clone());
            let status = Status::new();
            let scroll_list = ScrollList::new();
            let mut app = App {
                current_screen: Mode::FileChooser,
                exit: false,
                config,
                frame_rate: 30.0,
                tick_rate: 1.0,
                suspend: false,
                screens: vec![
                    Screen::new(Mode::FileChooser, vec![Box::new(list), Box::new(status)]),
                    Screen::new(Mode::ScriptRunner, vec![Box::new(scroll_list)]),
                ],
            };

            app.run().await?;

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
    let config_path_str = config_path.to_str().expect("Unknown host system").white();
    let version = env!("CARGO_PKG_VERSION").white();
    let version_msg = format!("Version: {}\n", version);
    let config_msg = format!("Config src: {}\n", config_path_str);
    execute!(
        stdout,
        Print("🦀 Aequitas Command And Control Console 🦀\n".yellow()),
        Print("\n"),
        Print(version_msg),
        Print("Edition: "),
        Print("Ultimate\n\n".white()),
        Print(config_msg)
    )?;

    stdout.flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let mut stdout = io::stdout();

    let config = read_config().expect("Error while loading config!");

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
                Err(ArgumentsError::PortNotNumber) => {
                    println!("ERROR: Supplied port is not a valid number");
                }
            };
        }
    }

    println!();
    Ok(())
}
