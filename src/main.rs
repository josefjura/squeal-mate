mod args;
mod border;
mod config;
mod db;
mod error;
mod list;
mod tui;
mod utils;

use args::{AeqArgs, Command};
use clap::Parser;
use config::{ensure_config_dir, read_config};
use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::Print,
    terminal::{self, enable_raw_mode, EnterAlternateScreen},
};
use db::Database;
use error::ArgumentsError;
use list::{Entry, Name};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Span,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, List, ListItem, ListState,
    },
    Frame,
};

use crossterm::{
    terminal::{disable_raw_mode, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::{
    collections::HashMap,
    error::Error,
    fs::read_dir,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use std::{
    fmt::Display,
    io::{self, stdout},
};

const MIN_HEIGHT: u16 = 8;
const MIN_WIDTH: u16 = 80;

struct CleanUp;

#[derive(PartialEq)]
enum Screen {
    FileChooser {
        entries: Vec<list::Entry>,
        state: ListState,
    },
    Quit,
}

struct App {
    base_path: PathBuf,
    current_screen: Screen,
    message: Message,
}

#[derive(Debug)]
enum Message {
    Success(String),
    Error(String),
    Info(String),
    Empty,
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Empty => write!(f, ""),
            Message::Error(text) => write!(f, "{}", text.clone().red()),
            Message::Success(text) => write!(f, "{}", text.clone().green()),
            Message::Info(text) => write!(f, "{}", text.clone().white()),
        }
    }
}

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

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

async fn process_events<'a>(
    display: &mut App,
    connection: &Database,
) -> Result<(), Box<dyn std::error::Error>> {
    while poll(Duration::ZERO)? {
        match read()? {
            // Event::Resize(x, y) => {
            //     display.window_height = y as usize;
            //     display.window_width = x as usize;
            //     if y < MIN_HEIGHT || x < MIN_WIDTH {
            //         display.error = Some("WINDOW TOO SMALL!".to_owned());
            //     } else {
            //         display.error = None;
            //         if let Screen::FileChooser(list, help) = &mut display.current_screen {
            //             list.resize(x as usize - help.width as usize, y as usize - 2)
            //         }
            //     }
            // }
            Event::Key(event)
                if event.code == KeyCode::Char('c')
                    && event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                display.current_screen = Screen::Quit
            }
            Event::Key(event) if event.code == KeyCode::Esc => {
                display.current_screen = Screen::Quit
            }
            Event::Key(event) if event.code == KeyCode::Char('q') => {
                display.current_screen = Screen::Quit
            }
            Event::Key(key) if key.kind == KeyEventKind::Press => match &mut display.current_screen
            {
                Screen::FileChooser { entries, state } => match key.code {
                    KeyCode::Up => state.cursor_up(),
                    KeyCode::Down => state.cursor_down(entries.len()),
                    KeyCode::Home => state.select(Some(0)),
                    KeyCode::End => state.select(Some(entries.len() - 1)),
                    KeyCode::Enter => {
                        if let Some(selected) = state.selected() {
                            let dir_name = entries.get(selected);
                            if let Some(entry) = dir_name {
                                match entry {
                                    Entry::Directory(dir_name) => {
                                        let new_path =
                                            display.base_path.join(std::path::Path::new(&dir_name));
                                        display.base_path = new_path;
                                        *entries = read_entries(&display.base_path);

                                        if entries.len() > 0 {
                                            state.select(Some(0))
                                        } else {
                                            state.select(None)
                                        }
                                    }
                                    Entry::File(_) => {}
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        let path = display.base_path.clone();
                        let old_path = path.as_path();
                        if let (Some(new_path), Some(old_dir)) =
                            (old_path.parent(), old_path.file_name())
                        {
                            display.base_path = new_path.to_path_buf();

                            *entries = read_entries(&display.base_path);
                            state.select(Some(0));

                            let old_index = entries
                                .iter()
                                .position(|r| r.get_name() == old_dir.to_str().unwrap());

                            if let Some(old_index) = old_index {
                                state.select(Some(old_index));
                            } else {
                                if entries.len() > 0 {
                                    state.select(Some(0))
                                } else {
                                    state.select(None)
                                }
                            }

                            //let _ = list.select(&old_dir.to_str().unwrap_or(""));
                        }
                    }
                    KeyCode::Char(char) => match char {
                        // 'a' => {
                        //     if let Some(Entry::File(file)) = list.get_selection() {
                        //         let full_path = display.base_path.join(Path::new(&file));
                        //         match connection.execute_script(full_path).await {
                        //             Err(e) => {
                        //                 eprintln!("{}", e)
                        //             }
                        //             _ => {}
                        //         }
                        //     }
                        // }
                        's' => {
                            if let Some(selected) = state.selected() {
                                if let Some(entry) = entries.get(selected) {
                                    if let Entry::File(file) = entry {
                                        let full_path = display.base_path.join(Path::new(&file));
                                        display.message = Message::Info("Executing script".into());
                                        match connection.execute_script(full_path).await {
                                            Err(e) => {
                                                display.message = Message::Error(e.to_string())
                                            }
                                            _ => {
                                                display.message =
                                                    Message::Success("Script execution done".into())
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    },
                    _ => (),
                    _ => {}
                },
                _ => (),
            },
            _ => (),
        }
    }

    Ok(())
}

impl ListController for ListState {
    fn cursor_up(&mut self) {
        if let Some(position) = self.selected() {
            if position > 0 {
                self.select(Some(position - 1))
            }
        }
    }

    fn cursor_down(&mut self, items_len: usize) {
        if let Some(position) = self.selected() {
            if position < items_len - 1 {
                self.select(Some(position + 1))
            }
        }
    }
}

trait ListController {
    fn cursor_up(&mut self);
    fn cursor_down(&mut self, items_len: usize);
}

async fn start_tui(
    rows: u16,
    cols: u16,
    config: &HashMap<String, String>,
    connection: &Database,
) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = tui::init()?;

    let list_height = rows as usize - 2;
    let path: PathBuf = if let Some(content) = config.get("path") {
        PathBuf::from(content)
    } else {
        PathBuf::from_str("./").expect("Can't open current directory")
    };

    let entries = read_entries(&path);

    let lines = [
        ("up/down", "move up and down"),
        ("lef/right", "page forward and back"),
        ("enter", "open directory"),
        ("backspace", "go level up"),
        ("a", "run all scripts since"),
        ("s", "run selected"),
        ("q/esc", "quit"),
    ];

    let mut state = App {
        base_path: path,
        current_screen: Screen::FileChooser {
            entries: entries.clone(),
            state: ListState::default().with_selected(Some(1)),
        },
        message: Message::Empty,
    };

    while state.current_screen != Screen::Quit {
        process_events(&mut state, connection).await?;

        terminal.draw(|frame| ui(frame, &mut state))?;

        //thread::sleep(Duration::from_millis(33));
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn ui(frame: &mut Frame, app_state: &mut App) {
    match &mut app_state.current_screen {
        Screen::FileChooser { entries, state } => {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(1),
                ])
                .split(frame.size());

            let list_draw = List::new(entries)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        // .title(
                        //     Title::from("Test")
                        //         .position(Position::Bottom)
                        //         .alignment(Alignment::Right)
                        //         .content(Span::styled("AEQ-CAC", Style::default().fg(Color::Red))),
                        // )
                        .border_type(BorderType::Double),
                )
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">>")
                .repeat_highlight_symbol(true);

            frame.render_widget(
                Span::styled(
                    app_state.base_path.to_str().unwrap_or(""),
                    Style::default().fg(Color::White),
                ),
                layout[0],
            );
            frame.render_stateful_widget(list_draw, layout[1], state);
            frame.render_widget(
                Span::styled(
                    format!("AEQ-CAC: {}", app_state.message),
                    Style::default().fg(Color::Red),
                ),
                layout[2],
            );
        }
        _ => {}
    }
}
impl<'a> From<&mut Entry> for ListItem<'a> {
    fn from(value: &mut Entry) -> Self {
        let style = match (value) {
            Entry::File(_) => Style::new().white(),
            Entry::Directory(_) => Style::new().blue(),
        };

        ListItem::<'a>::new(value.get_name().to_string()).style(style)
    }
}

fn draw_help(stdout: &mut io::Stdout) -> Result<(), Box<dyn std::error::Error>> {
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
    let _clean_up = CleanUp;
    let mut stdout = io::stdout();

    let config = read_config().expect("Error while loading config!");

    let (cols, rows) = terminal::size()?;

    let args = AeqArgs::parse();

    match args.command {
        Some(Command::Config) => {
            draw_help(&mut stdout)?;
        }
        Some(Command::Migrations) | None => {
            match args.connection.merge(&config) {
                Ok(conn) => start_tui(rows, cols, &config, &conn).await?,
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
