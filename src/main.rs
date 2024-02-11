mod config;
mod utils;

use config::setup_config;
use crossterm::{
    cursor::{DisableBlinking, Hide, MoveTo},
    event::{poll, read, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Print, Stylize},
    terminal::{self, Clear, ClearType},
};
use std::{
    fs,
    io::{self, Error, Write},
    path::PathBuf,
    thread,
    time::Duration,
};
use utils::{read_and_validate_path, round_up_division};

const MIN_HEIGHT: u16 = 3;
struct CleanUp;

#[derive(PartialEq, Debug)]
struct SelectionState {
    cursor: usize,
    current_row_count: usize,
    row_count: usize,
    current_page: usize,
    page_count: usize,
    entries: Vec<String>,
}

impl SelectionState {
    fn move_cursor_up(&self) -> Self {
        let new_cursor = if self.cursor == 0 {
            self.current_row_count - 1
        } else {
            self.cursor - 1
        };
        Self {
            cursor: new_cursor,
            entries: self.entries.clone(),
            ..*self
        }
    }

    fn move_cursor_down(&self) -> Self {
        let new_cursor = if self.cursor == self.current_row_count - 1 {
            0
        } else {
            self.cursor + 1
        };
        Self {
            cursor: new_cursor,
            entries: self.entries.clone(),
            ..*self
        }
    }

    fn move_page_forward(&self) -> Self {
        let new_page = if self.current_page == self.page_count - 1 {
            0
        } else {
            self.current_page + 1
        };

        let displayed_items = get_list_size(self.entries.len(), self.row_count, new_page);

        Self {
            current_page: new_page,
            entries: self.entries.clone(),
            cursor: self.reset_cursor(displayed_items),
            current_row_count: displayed_items,
            ..*self
        }
    }

    fn move_page_back(&self) -> Self {
        let new_page = if self.current_page == 0 {
            self.page_count - 1
        } else {
            self.current_page - 1
        };

        let displayed_items = get_list_size(self.entries.len(), self.row_count, new_page);

        Self {
            current_page: new_page,
            entries: self.entries.clone(),
            cursor: self.reset_cursor(displayed_items),
            current_row_count: displayed_items,
            ..*self
        }
    }

    fn reset_cursor(&self, new_row_count: usize) -> usize {
        let new_cursor = if self.cursor >= new_row_count {
            new_row_count - 1
        } else {
            self.cursor
        };
        new_cursor
    }

    fn resize(&self, size: usize) -> Self {
        let current_row_count = get_list_size(self.entries.len(), size, self.current_page);
        let page_count = round_up_division(self.entries.len(), size);

        Self {
            current_page: 0,
            cursor: 0,
            page_count,
            current_row_count,
            row_count: size,
            entries: self.entries.clone(),
            ..*self
        }
    }
}

fn get_list_size(entries_count: usize, size: usize, current_page: usize) -> usize {
    let list_size = std::cmp::min(size, entries_count.saturating_sub(size * current_page));

    list_size
}

#[derive(PartialEq)]
enum AppState {
    Selection(SelectionState),
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
    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries
            .filter_map(|entry| {
                entry
                    .ok()
                    .and_then(|e| e.path().file_name()?.to_str()?.to_owned().into())
            })
            .filter(|x| !x.starts_with("_") && !x.starts_with("."))
            .collect(),
        Err(e) => {
            eprintln!("Failed to read directory: {}", e);
            Vec::new()
        }
    };

    entries
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
                    if let AppState::Selection(selection) = &display.state {
                        display.state = AppState::Selection(selection.resize(y as usize))
                    }
                }
            }
            Event::Key(event)
                if event.code == KeyCode::Char('c')
                    && event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                display.state = AppState::Quit
            }
            Event::Key(key) => match &display.state {
                AppState::Selection(selection) => match key.code {
                    KeyCode::Up => display.state = AppState::Selection(selection.move_cursor_up()),
                    KeyCode::Down => {
                        display.state = AppState::Selection(selection.move_cursor_down())
                    }
                    KeyCode::Left => {
                        display.state = AppState::Selection(selection.move_page_back())
                    }
                    KeyCode::Right => {
                        display.state = AppState::Selection(selection.move_page_forward())
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
        Print(format!("WINDOW TOO SMALL").red())
    )?;

    Ok(())
}

fn draw_selection(
    display: &Display,
    selection: &SelectionState,
    stdout: &mut io::Stdout,
) -> Result<(), Error> {
    for line in 0..selection.row_count {
        let line_index = (selection.current_page * selection.row_count) + line;
        let item = selection.entries.get(line_index);

        match item {
            Some(entry) => {
                let entry_clone = entry.clone();
                if selection.cursor == line {
                    queue!(
                        stdout,
                        MoveTo(0, line as u16),
                        Clear(ClearType::CurrentLine),
                        Print(format!(" > {entry_clone}").blue())
                    )?
                } else {
                    queue!(
                        stdout,
                        MoveTo(0, line as u16),
                        Clear(ClearType::CurrentLine),
                        Print(format!("   {entry_clone}").white())
                    )?
                }
            }
            None => queue!(
                stdout,
                MoveTo(0, line as u16),
                Clear(ClearType::CurrentLine),
            )?,
        }
    }

    queue!(
        stdout,
        MoveTo(0, display.window_height as u16 - 2),
        Clear(ClearType::CurrentLine)
    )?;
    let text = format!(
        "Page {}/{}",
        selection.current_page + 1,
        selection.page_count
    );
    // let text = format!(
    //     "Page {}/{} (window: {}/full: {}/current: {}) entries: {}, cursor: {}",
    //     selection.current_page + 1,
    //     selection.page_count,
    //     display.window_height,
    //     selection.row_count,
    //     selection.current_row_count,
    //     selection.entries.len(),
    //     selection.cursor + 1
    // );
    queue!(
        stdout,
        MoveTo(0, display.window_height as u16 - 1),
        Clear(ClearType::CurrentLine),
        Print(text)
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
    let page_count = round_up_division(entries.len(), row_count);

    let mut display = Display {
        window_height: rows as usize,
        error: None,
        state: AppState::Selection(SelectionState {
            current_page: 0,
            cursor: 0,
            current_row_count: std::cmp::min(row_count, entries.len()),
            row_count,
            page_count,
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
