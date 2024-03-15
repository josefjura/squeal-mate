mod app;
mod args;
mod config;
mod db;
mod entries;
mod error;
mod event;
mod handler;
mod tui;
mod ui;

use args::{AeqArgs, Command};
use clap::Parser;
use config::{ensure_config_dir, read_config};

use crossterm::{execute, style::Print};
use db::Database;
use entries::{Entry, Name};
use error::ArgumentsError;
use event::{Event, EventHandler};
use handler::handle_key_events;
use ratatui::{
    backend::CrosstermBackend,
    style::{Style, Stylize},
    widgets::{ListItem, ListState},
    Terminal,
};

use tui::Tui;

use std::io::{self};
use std::{
    collections::HashMap,
    error::Error,
    fs::read_dir,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::app::{App, Screen, UiState};

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

async fn start_tui(
    config: &HashMap<String, String>,
    connection: Database,
) -> Result<(), Box<dyn Error>> {
    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    let path: PathBuf = if let Some(content) = config.get("path") {
        PathBuf::from(content)
    } else {
        PathBuf::from_str("./").expect("Can't open current directory")
    };

    let entries = read_entries(&path);

    let mut app = App {
        base_path: path,
        current_screen: Screen::FileChooser {
            entries: entries.clone(),
        },
        message: None,
        connection,
        exit: false,
        ui_state: UiState {
            list: ListState::default().with_selected(Some(1)),
        },
    };

    // Start the main loop.
    while !app.exit {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app).await?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}

#[derive(Clone)]
pub enum Action {
    Tick,
    Increment,
    Decrement,
    NetworkRequestAndThenIncrement, // new
    NetworkRequestAndThenDecrement, // new
    Quit,
    Render,
    None,
}

impl<'a> From<&Entry> for ListItem<'a> {
    fn from(value: &Entry) -> Self {
        let style = match value {
            Entry::File(_) => Style::new().white(),
            Entry::Directory(_) => Style::new().blue(),
        };

        ListItem::<'a>::new(value.get_name().to_string()).style(style)
    }
}

fn draw_help(stdout: &mut io::Stdout) -> Result<(), Box<dyn Error>> {
    let config_path = ensure_config_dir()?;
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
async fn main() -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout();

    let config = read_config().expect("Error while loading config!");

    let args = AeqArgs::parse();

    match args.command {
        Some(Command::Config) => {
            draw_help(&mut stdout)?;
        }
        Some(Command::Migrations) | None => {
            match args.connection.merge(&config) {
                Ok(conn) => start_tui(&config, conn).await?,
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
