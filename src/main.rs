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
    fs::{self},
    io::{self, Error, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;
use utils::read_and_validate_path;

const MIN_HEIGHT: u16 = 8;
const MIN_WIDTH: u16 = 60;

struct CleanUp;

#[derive(PartialEq)]
enum AppState {
    Selection(FileList),
    Quit,
}

struct Display {
    window_width: usize,
    window_height: usize,
    base_path: PathBuf,
    error: Option<String>,
    state: AppState,
}

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

static KEY_BINDINGS: &[(&str, &str)] = &[
    ("up/down", "move up and down"),
    ("lef/right", "page forward and back"),
    ("enter", "open directory"),
    ("backspace", "go level up"),
    ("a", "run all scripts since"),
    ("s", "run selected"),
];

#[tokio::test]
async fn print_script_test() {
    let input = Path::new("/mnt/c/Users/josef/source/eurowag/Aequitas/Database/Migrates/db 34/db 34.8/V20231214.02__T023-818__T023-4142_Translations.sql");
    match print_script(input.to_path_buf()).await {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("{}", e),
    }
    // let script = tokio::fs::read_to_string(input).await;

    //assert!(script.is_ok());
}

async fn print_script(path: PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let script = tokio::fs::read_to_string(path).await;
    let mut config = Config::new();

    config.host("127.0.0.1");
    config.port(1433);
    config.authentication(AuthMethod::sql_server("cli", "clipassword"));
    config.trust_cert(); // on production, it is not a good idea to do this

    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;

    // To be able to use Tokio's tcp, we're using the `compat_write` from
    // the `TokioAsyncWriteCompatExt` to get a stream compatible with the
    // traits from the `futures` crate.
    let mut client = Client::connect(config, tcp.compat_write()).await?;

    let result = client.simple_query(script.unwrap()).await;

    match result {
        Ok(_) => Ok("Finished".to_owned()),
        Err(err) => Ok(err.to_string()),
    }
}

fn read_entries(path: &Path) -> Vec<Entry> {
    let mut entries = match fs::read_dir(path) {
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

async fn process_events(display: &mut Display) -> Result<(), Box<dyn std::error::Error>> {
    while poll(Duration::ZERO)? {
        match read()? {
            Event::Resize(x, y) => {
                display.window_height = y as usize;
                display.window_width = x as usize;
                if y < MIN_HEIGHT || x < MIN_WIDTH {
                    display.error = Some(String::from("WINDOW TOO SMALL!"));
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
                AppState::Selection(list) => match key.code {
                    KeyCode::Up => list.move_cursor_up(),
                    KeyCode::Down => list.move_cursor_down(),
                    KeyCode::Left => list.move_page_back(),
                    KeyCode::Right => list.move_page_forward(),
                    KeyCode::Enter => {
                        let dir_name = list.get_selection();
                        if let Some(entry) = dir_name {
                            match entry {
                                Entry::Directory(dir_name) => {
                                    let new_path =
                                        display.base_path.join(std::path::Path::new(dir_name));
                                    display.base_path = new_path;
                                    list.set_entries(read_entries(&display.base_path));
                                }
                                Entry::File(_) => {}
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        let new_path = display.base_path.join(std::path::Path::new(".."));
                        display.base_path = new_path;
                        list.set_entries(read_entries(&display.base_path));
                    }
                    KeyCode::Char(char) => match char {
                        'a' => {
                            if let Some(Entry::File(file)) = list.get_selection() {
                                let full_path = display.base_path.join(Path::new(file));
                                print_script(full_path).await?;
                            }
                        }
                        's' => {
                            if let Some(Entry::File(file)) = list.get_selection() {
                                let full_path = display.base_path.join(Path::new(file));
                                print_script(full_path).await?;
                            }
                        }
                        _ => (),
                    },
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
    const ERROR_TEXT: &str = "WINDOW TOO SMALL";
    queue!(
        stdout,
        MoveTo(
            ((display.window_width / 2) - (ERROR_TEXT.len() / 2)) as u16,
            (display.window_height / 2) as u16
        ),
        Clear(ClearType::All),
        Print(ERROR_TEXT.red())
    )?;

    Ok(())
}

fn draw_list(stdout: &mut std::io::Stdout, list: &FileList) -> Result<(), std::io::Error> {
    let page = list.get_page_entries();

    for line in 0..list.height {
        if let Some(item) = page.get(line) {
            match item {
                Entry::Directory(dir) => {
                    if line == list.cursor {
                        queue!(
                            stdout,
                            MoveTo(0, line as u16),
                            Print(format!(" > {}", dir).black().on_white()),
                            Clear(ClearType::UntilNewLine)
                        )?;
                    } else {
                        queue!(
                            stdout,
                            MoveTo(0, line as u16),
                            Print(format!("   {}", dir).blue()),
                            Clear(ClearType::UntilNewLine)
                        )?;
                    }
                }
                Entry::File(file) => {
                    if line == list.cursor {
                        queue!(
                            stdout,
                            MoveTo(0, line as u16),
                            Print(format!(" > {}", file).black().on_white()),
                            Clear(ClearType::UntilNewLine)
                        )?;
                    } else {
                        queue!(
                            stdout,
                            MoveTo(0, line as u16),
                            Print(format!("   {}", file).white()),
                            Clear(ClearType::UntilNewLine)
                        )?;
                    }
                }
            }
        } else {
            queue!(
                stdout,
                MoveTo(0, line as u16),
                Clear(ClearType::CurrentLine)
            )?;
        }
    }

    Ok(())
}

fn draw_rect(
    stdout: &mut io::Stdout,
    display: &Display,
    help_lines: &[(&str, &str)],
) -> Result<(), Error> {
    const SPLITTER: &str = " : ";
    let width: u16 = help_lines
        .iter()
        .map(|f| f.0.len() + f.1.len() + SPLITTER.len())
        .max()
        .unwrap_or(10) as u16
        + 4;
    let height: u16 = help_lines.len() as u16 + 2;
    let row: u16 = 0;
    let column: u16 = display.window_width as u16 - width;
    let tl = (column, row);
    let tr = (column + width - 1, row);
    let bl = (column, height - 1 + row);
    let br = (column + width - 1, height - 1 + row);

    // ┌─┐
    // │ │
    // └─┘

    queue!(stdout, MoveTo(tl.0, tl.1), Print("┌".yellow()))?;
    queue!(stdout, MoveTo(tr.0, tr.1), Print("┐".yellow()))?;
    queue!(stdout, MoveTo(bl.0, br.1), Print("└".yellow()))?;
    queue!(stdout, MoveTo(br.0, br.1), Print("┘".yellow()))?;

    for line in tl.0 + 1..tr.0 {
        queue!(stdout, MoveTo(line, tl.1 as u16), Print("─".yellow()))?;
        queue!(stdout, MoveTo(line, bl.1 as u16), Print("─".yellow()))?;
    }

    for col in tl.1 + 1..bl.1 {
        queue!(stdout, MoveTo(tl.0 as u16, col), Print("│".yellow()))?;
        queue!(stdout, MoveTo(tr.0 as u16, col), Print("│".yellow()))?;
    }

    for text in help_lines.iter().enumerate() {
        let (index, (label, value)) = text;
        queue!(
            stdout,
            MoveTo(column + 2, row + 1 + index as u16),
            Print(label.white()),
            Print(SPLITTER),
        )?;
        queue!(
            stdout,
            MoveTo(
                column + width - 2 - value.len() as u16,
                row + 1 + index as u16
            ),
            Print(value.yellow())
        )?;
    }

    Ok(())
}

fn draw_selection(
    display: &Display,
    list: &FileList,
    stdout: &mut io::Stdout,
) -> Result<(), Error> {
    draw_list(stdout, list)?;

    draw_rect(stdout, display, KEY_BINDINGS)?;

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
            list.page_index + 1,
            list.get_page_count()
        )),
        Clear(ClearType::UntilNewLine)
    )?;

    Ok(())
}

fn draw(stdout: &mut io::Stdout, display: &Display) -> Result<(), Error> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _clean_up = CleanUp;
    let mut stdout = io::stdout();

    execute!(&mut stdout, Clear(ClearType::All))?;

    let config = setup_config();

    execute!(&mut stdout, Hide, DisableBlinking)?;
    stdout.flush()?;

    let path: PathBuf = read_and_validate_path(&mut stdout, config);

    let (cols, rows) = terminal::size()?;

    terminal::enable_raw_mode()?;

    let entries = read_entries(&path);

    let row_count = rows as usize - 2;

    let mut display = Display {
        window_height: rows as usize,
        window_width: cols as usize,
        error: None,
        base_path: path,
        state: AppState::Selection(FileList {
            height: row_count,
            page_index: 0,
            cursor: 0,
            entries,
        }),
    };

    while display.state != AppState::Quit {
        process_events(&mut display).await?;
        draw(&mut stdout, &display)?;

        thread::sleep(Duration::from_millis(33));
    }

    Ok(())
}
