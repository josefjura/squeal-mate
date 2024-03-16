mod action;
mod app;
mod cli;
mod components;
mod config;
mod db;
mod entries;
mod error;
mod tui;
mod utils;

use crate::app::{App, Mode, UiState};
use crate::components::list::List;
use clap::Parser;
use cli::{AeqArgs, Command};
use color_eyre::eyre;
use components::status::Status;
use config::{get_config_dir, read_config};
use crossterm::{execute, style::Print};
use db::Database;
use entries::Entry;
use error::ArgumentsError;
use ratatui::{style::Stylize, widgets::ListState};
use std::io::{self};
use std::{
    collections::HashMap,
    fs::read_dir,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};
use utils::{initialize_logging, initialize_panic_handler};

fn read_entries(path: &Path) -> Vec<Entry> {
    let mut entries = match read_dir(path) {
        Ok(entries) => entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let file_name = path.file_name()?.to_str()?;

                if file_name.starts_with('_') || file_name.starts_with('.') {
                    return None;
                }

                // Check if it's a directory or a file with .sql extension
                if path.is_dir() {
                    Some(Entry::Directory(file_name.to_owned()))
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("sql") {
                    Some(Entry::File(file_name.to_owned()))
                } else {
                    None
                }
            })
            .collect(),
        Err(e) => {
            eprintln!("Failed to read directory: {}", e);
            Vec::new()
        }
    };

    entries.sort();

    entries
}

async fn start_tui(config: HashMap<String, String>, connection: Database) -> eyre::Result<()> {
    initialize_logging()?;

    initialize_panic_handler()?;

    let path: PathBuf = if let Some(content) = config.get("path") {
        PathBuf::from(content)
    } else {
        PathBuf::from_str("./").expect("Can't open current directory")
    };

    let entries = read_entries(&path);
    let list = List::new(entries, path, connection.clone());
    let status = Status::new();
    let mut app = App {
        current_screen: Mode::FileChooser,
        connection,
        exit: false,
        ui_state: UiState {
            list: ListState::default().with_selected(Some(1)),
        },
        config,
        frame_rate: 30.0,
        tick_rate: 1.0,
        suspend: false,
        components: vec![Box::new(list), Box::new(status)],
    };

    app.run().await?;

    Ok(())
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
