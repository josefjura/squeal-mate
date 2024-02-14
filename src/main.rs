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
use list::List;
use std::{
    fs,
    io::{self, Error, Write},
    path::PathBuf,
    thread,
    time::Duration,
};
use utils::read_and_validate_path;

const MIN_HEIGHT: u16 = 3;
struct CleanUp;

#[derive(PartialEq)]
enum AppState {
    Selection(List),
    Quit,
}

struct Display {
    window_height: usize,
    error: Option<String>,
    state: AppState,
}

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

fn read_entries(path: &PathBuf) -> Vec<String> {
    match fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(|entry| {
                entry
                    .ok()
                    .and_then(|e| e.path().file_name()?.to_str()?.to_owned().into())
            })
            .filter(|x| !x.starts_with('_') && !x.starts_with('.'))
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
    selection: &List,
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
        state: AppState::Selection(List {
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
