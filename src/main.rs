use crossterm::{
    cursor::{self, MoveTo},
    event::{poll, read, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Print, Stylize},
    terminal::{self, Clear, ClearType},
};
use std::{
    cmp, fs,
    io::{self, Write},
    path::Path,
    thread,
    time::Duration,
};

struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

fn main() -> io::Result<()> {
    let _clean_up = CleanUp;
    let mut stdout = io::stdout();

    execute!(stdout, cursor::Hide, Clear(ClearType::All))?;

    let (_, rows) = terminal::size()?;
    let _ = terminal::enable_raw_mode();

    let mut quit = false;
    let mut selection: usize = 0;
    let mut page: usize = 0;
    let mut page_size: usize = rows as usize - 2;

    let mut parts = Vec::<String>::new();

    while !quit {
        let base_path = Path::new("/mnt/c/Users/josef/source/eurowag/Aequitas");
        let mut path = base_path.join("Database/Migrates");

        for part in &parts {
            path = path.join(part);
        }

        let info = std::fs::metadata(&path).unwrap();

        if info.is_dir() {
            let entries = match fs::read_dir(&path) {
                Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
                Err(e) => {
                    eprintln!("Failed to read directory: {}", e);
                    Vec::new()
                }
            };
            let entry_count = entries.len();

            //let mut real_items = data.into_iter().peekable();

            let page_count: usize = entry_count / page_size;
            let current_page_size: usize = cmp::min(page_size, entry_count - (page * page_size));

            if selection > current_page_size {
                selection = current_page_size - 1;
            }

            while poll(Duration::ZERO)? {
                match read()? {
                    Event::Key(event) => {
                        if event.kind == KeyEventKind::Press {
                            match event.code {
                                KeyCode::Char(x)
                                    if event.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    match x {
                                        'c' => quit = true,
                                        _ => {}
                                    }
                                }
                                KeyCode::Up => match selection {
                                    0 => selection = current_page_size - 1,
                                    _ => selection -= 1,
                                },
                                KeyCode::Down => match selection {
                                    x if x == current_page_size - 1 => selection = 0,
                                    _ => selection += 1,
                                },
                                KeyCode::Left => match page {
                                    0 => page = page_count,
                                    _ => page -= 1,
                                },
                                KeyCode::Right => match page {
                                    _ if page == page_count => page = 0,
                                    _ => page += 1,
                                },
                                KeyCode::Enter => {
                                    let item_index = page * page_size + selection;
                                    let item = entries.get(item_index);
                                    if let Some(value) = item {
                                        if let Ok(path) = value.file_name().into_string() {
                                            parts.push(path);
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    parts.pop();
                                }
                                KeyCode::Esc => quit = true,
                                _ => {}
                            }
                        }
                    }
                    Event::Resize(_, y) => {
                        page_size = y as usize - 2;
                    }
                    _ => (),
                }
            }

            for line in 0..page_size {
                let line_index = page * page_size + line;
                let item = entries.get(line_index);

                match item {
                    Some(entry) => match entry.file_name().into_string() {
                        Ok(inner) => {
                            if selection == line {
                                queue!(
                                    stdout,
                                    MoveTo(0, line as u16),
                                    Clear(ClearType::CurrentLine),
                                    Print(format!(" >{inner}").blue())
                                )?
                            } else {
                                queue!(
                                    stdout,
                                    MoveTo(0, line as u16),
                                    Clear(ClearType::CurrentLine),
                                    Print(format!("  {inner}").white())
                                )?
                            }
                        }
                        Err(_) => queue!(
                            stdout,
                            MoveTo(0, line as u16),
                            Clear(ClearType::CurrentLine),
                            Print("ERROR".red()),
                        )?,
                    },
                    None => queue!(
                        stdout,
                        MoveTo(0, line as u16),
                        Clear(ClearType::CurrentLine),
                    )?,
                }
            }

            let text = format!("Page {page}/{page_count} ({entry_count})");
            queue!(
                stdout,
                MoveTo(0, page_size as u16 + 1),
                Clear(ClearType::CurrentLine),
                Print(text)
            )?;
        }

        stdout.flush()?;

        thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}
