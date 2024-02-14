mod config;
mod list;
mod utils;

use config::setup_config;
use crossterm::{
    cursor::{DisableBlinking, Hide, MoveTo},
    event::{poll, read, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Print, Stylize},
    terminal::{self, Clear, ClearType},
};
use list::{Entry, FileList};
use std::{
    collections::HashMap,
    fs,
    io::{self, Error, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use utils::read_and_validate_path;

const MIN_HEIGHT: u16 = 3;
struct CleanUp;

#[derive(PartialEq)]
enum AppState {
    Selection(FileList),
    Quit,
}

struct Display {
    window_height: usize,
    path: PathBuf,
    error: Option<String>,
    state: AppState,
}

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

// #[test]
// fn test() {
//     let prd = "/mnt/c/Users/josef/source/eurowag/Aequitas/Database/Migrates/db 24/db 24.7";
//     let entries = read_entries(Path::new(prd));

//     let debug = entries;
// }

fn read_entries(path: &Path) -> Vec<Entry> {
    match fs::read_dir(path) {
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
    }
}

fn process_events(display: &mut Display) -> Result<(), Error> {
    while poll(Duration::ZERO)? {
        match read()? {
            Event::Resize(_, y) => {
                display.window_height = y as usize;
                if y < MIN_HEIGHT {
                    display.error = Some(String::from("Fuck you!"));
                } else {
                    display.error = None;
                    if let AppState::Selection(selection) = &mut display.state {
                        selection.resize(y as usize - 2)
                    }
                }
            }
            Event::Key(event)
                if event.code == KeyCode::Char('c')
                    && event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                display.state = AppState::Quit
            }
            Event::Key(key) => match &mut display.state {
                AppState::Selection(selection) => match key.code {
                    KeyCode::Up => selection.move_cursor_up(),
                    KeyCode::Down => selection.move_cursor_down(),
                    KeyCode::Left => selection.move_page_back(),
                    KeyCode::Right => selection.move_page_forward(),
                    KeyCode::Enter => {
                        let dir_name = selection.get_selection();
                        if let Some(dir_name) = dir_name {
                            let new_path =
                                display.path.join(std::path::Path::new(dir_name.get_name()));
                            display.path = new_path;
                            selection.set_entries(read_entries(&display.path));
                        }
                    }
                    KeyCode::Backspace => {
                        let new_path = display.path.join(std::path::Path::new(".."));
                        display.path = new_path;
                        selection.set_entries(read_entries(&display.path));
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
    }

    Ok(())
}

fn draw_error(display: &Display, stdout: &mut io::Stdout) -> Result<(), Error> {
    queue!(
        stdout,
        MoveTo(0, display.window_height as u16 - 1),
        Clear(ClearType::CurrentLine),
        Print("WINDOW TOO SMALL".red())
    )?;

    Ok(())
}

fn draw_selection(
    display: &Display,
    selection: &FileList,
    stdout: &mut io::Stdout,
) -> Result<(), Error> {
    let _ = selection.draw(stdout);

    queue!(
        stdout,
        MoveTo(0, display.window_height as u16 - 2),
        Clear(ClearType::CurrentLine)
    )?;

    queue!(
        stdout,
        MoveTo(0, display.window_height as u16 - 1),
        Print(format!(
            "AEQ-CAC SQL ({}/{})",
            selection.page_index + 1,
            selection.get_page_count()
        )),
        Clear(ClearType::UntilNewLine)
    )?;

    Ok(())
}

fn draw(display: &Display, stdout: &mut io::Stdout) -> Result<(), Error> {
    if display.error.is_some() {
        draw_error(display, stdout)?;
    } else {
        match &display.state {
            AppState::Selection(selection) => {
                draw_selection(display, selection, stdout)?;
            }
            _ => (),
        }
    }

    stdout.flush()?;
    Ok(())
}

fn main() -> io::Result<()> {
    let _clean_up = CleanUp;
    let mut stdout = io::stdout();

    execute!(&mut stdout, Clear(ClearType::All))?;

    let config = setup_config();

    execute!(&mut stdout, Hide, DisableBlinking)?;
    stdout.flush()?;

    let path: PathBuf = read_and_validate_path(&mut stdout, config);

    let (_, rows) = terminal::size()?;

    terminal::enable_raw_mode()?;

    let entries = read_entries(&path);

    let row_count = rows as usize - 2;

    let mut display = Display {
        window_height: rows as usize,
        error: None,
        path,
        state: AppState::Selection(FileList {
            height: row_count,
            page_index: 0,
            cursor: 0,
            entries,
        }),
    };

    while display.state != AppState::Quit {
        process_events(&mut display)?;
        draw(&display, &mut stdout)?;

        thread::sleep(Duration::from_millis(33));
    }

    Ok(())
}
