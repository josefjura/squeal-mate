mod border;
mod config;
mod list;
mod utils;

use border::draw_rect;
use config::setup_config;
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, Hide, MoveTo, MoveToNextLine, Show},
    event::{poll, read, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Print, Stylize},
    terminal::{self, Clear, ClearType},
};

use list::{Entry, FileList, Name};
use std::{
    fs,
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
const MIN_WIDTH: u16 = 80;

struct CleanUp;

#[derive(PartialEq)]
struct Help<'a> {
    width: u16,
    height: u16,
    padding: (u16, u16),
    spacer: char,
    lines: &'a [(&'a str, &'a str)],
}

impl<'a> Help<'a> {
    fn create(lines: &'a [(&'a str, &'a str)], padding: (u16, u16), spacer: char) -> Help {
        const BORDER_WIDTH: u16 = 1;
        const SPLITTER_WIDTH: u16 = 3;
        let width: u16 =
            get_max_length(lines) + (2 * BORDER_WIDTH) + (2 * padding.0) + SPLITTER_WIDTH;
        let height: u16 = lines.len() as u16 + (2 * BORDER_WIDTH) + (2 * padding.1);

        Self {
            lines,
            padding,
            spacer,
            width,
            height,
        }
    }
}

#[derive(PartialEq)]
enum AppState<'a> {
    Selection(FileList, Help<'a>),
    Quit,
}

struct Display<'a> {
    window_width: usize,
    window_height: usize,
    base_path: PathBuf,
    error: Option<String>,
    state: AppState<'a>,
}

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode")
    }
}

fn get_max_length<'a>(lines: &'a [(&'a str, &'a str)]) -> u16 {
    lines
        .iter()
        .map(|f| f.0.len() + f.1.len())
        .max()
        .unwrap_or(10) as u16
}

#[tokio::test]
async fn print_script_test() -> () {
    let input = Path::new("/mnt/c/Users/josef/source/eurowag/Aequitas/Database/Migrates/db 34/db 34.8/V20231214.02__T023-818__T023-4142_Translations.sql");
    match print_script(input.to_path_buf()).await {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("{}", e),
    }
    // let script = tokio::fs::read_to_string(input).await;

    // assert!(script.is_ok());

    ()
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

async fn process_events<'a>(display: &mut Display<'a>) -> Result<(), Box<dyn std::error::Error>> {
    while poll(Duration::ZERO)? {
        match read()? {
            Event::Resize(x, y) => {
                display.window_height = y as usize;
                display.window_width = x as usize;
                if y < MIN_HEIGHT || x < MIN_WIDTH {
                    display.error = Some("WINDOW TOO SMALL!".to_owned());
                } else {
                    display.error = None;
                    if let AppState::Selection(list, help) = &mut display.state {
                        list.resize(x as usize - help.width as usize, y as usize - 2)
                    }
                }
            }
            Event::Key(event)
                if event.code == KeyCode::Char('c')
                    && event.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                display.state = AppState::Quit
            }
            Event::Key(event) if event.code == KeyCode::Esc => display.state = AppState::Quit,
            Event::Key(event) if event.code == KeyCode::Char('q') => display.state = AppState::Quit,
            Event::Key(key) if key.kind == KeyEventKind::Press => match &mut display.state {
                AppState::Selection(list, _) => match key.code {
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
                        let path = display.base_path.clone();
                        let old_path = path.as_path();
                        if let (Some(new_path), Some(old_dir)) =
                            (old_path.parent(), old_path.file_name())
                        {
                            display.base_path = new_path.to_path_buf();

                            list.set_entries(read_entries(&display.base_path));
                            let _ = list.select(&old_dir.to_str().unwrap_or(""));
                        }
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

fn clamp_string(s: &str, max_length: usize) -> String {
    match s.char_indices().nth(max_length) {
        Some((idx, _)) => String::from(&s[..idx]),
        None => s.into(),
    }
}

fn draw_list(stdout: &mut std::io::Stdout, list: &FileList) -> Result<(), std::io::Error> {
    let page = list.get_page_entries();

    for line in 0..list.height {
        if let Some(item) = page.get(line) {
            let name = item.get_name();
            let is_selected = line == list.cursor;

            // TODO: cleanup
            let res = match (item, is_selected) {
                (_, true) => format!(" > {}", &name),
                (Entry::File(_), _) => format!("   {}", &name),
                (Entry::Directory(_), _) => format!("   {}", &name),
            };

            let clamped = clamp_string(&res, list.width - 1).to_owned();

            let styled_text = match (item, is_selected) {
                (_, true) => clamped.clone().black().on_white().to_string(),
                (Entry::File(_), _) => clamped.clone().white().to_string(),
                (Entry::Directory(_), _) => clamped.clone().blue().to_string(),
            };

            queue!(
                stdout,
                MoveTo(0, line as u16),
                Print(styled_text),
                Print(" ".repeat(list.width - clamped.len()))
            )?;
        } else {
            queue!(
                stdout,
                MoveTo(0, line as u16),
                Print(" ".repeat(list.width))
            )?;
        }
    }

    Ok(())
}

fn draw_help(stdout: &mut io::Stdout, display: &Display, help: &Help) -> Result<(), Error> {
    const SPLITTER: &str = " : ";

    let row: u16 = 0;
    let column: u16 = display.window_width as u16 - help.width;

    draw_rect(stdout, column, 0, help.width, help.height)?;

    for text in help.lines.iter().enumerate() {
        let (index, (label, value)) = text;
        queue!(
            stdout,
            MoveTo(column + 2, row + 1 + index as u16),
            Print(label.white()),
            Print(SPLITTER),
            MoveTo(
                column + help.width - 2 - value.len() as u16,
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
    help: &Help,
    stdout: &mut io::Stdout,
) -> Result<(), Error> {
    draw_list(stdout, list)?;

    draw_help(stdout, display, help)?;

    let prompt = "AEQ-CAC >";
    let text = if let Some(s) = list.get_selection() {
        format!("{} {}", prompt, s)
    } else {
        "".to_owned()
    };

    let text2 = clamp_string(&text, display.window_width);

    queue!(
        stdout,
        MoveTo(0, display.window_height as u16 - 1),
        Print(text2),
        Clear(ClearType::UntilNewLine)
    )?;

    Ok(())
}

fn draw(stdout: &mut io::Stdout, display: &Display) -> Result<(), Error> {
    if display.error.is_some() {
        draw_error(display, stdout)?;
    } else {
        match &display.state {
            AppState::Selection(list, help) => {
                draw_selection(display, list, help, stdout)?;
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

    let args: Vec<String> = std::env::args().collect();

    execute!(&mut stdout, Clear(ClearType::All))?;

    let config = setup_config().expect("Error while loading config!");

    execute!(&mut stdout, Hide, DisableBlinking)?;
    stdout.flush()?;

    let lines = [
        ("up/down", "move up and down"),
        ("lef/right", "page forward and back"),
        ("enter", "open directory"),
        ("backspace", "go level up"),
        ("a", "run all scripts since"),
        ("s", "run selected"),
        ("q/esc", "quit"),
    ];

    let help = Help::create(&lines, (1, 0), ':');

    let path: PathBuf = read_and_validate_path(config);

    let (cols, rows) = terminal::size()?;

    terminal::enable_raw_mode()?;

    let entries = read_entries(&path);

    let row_count = rows as usize - 2;

    let mut display = Display {
        window_height: rows as usize,
        window_width: cols as usize,
        error: None,
        base_path: path,
        state: AppState::Selection(
            FileList {
                height: row_count,
                width: (cols - help.width - 1) as usize,
                page_index: 0,
                cursor: 0,
                entries,
            },
            help,
        ),
    };

    while display.state != AppState::Quit {
        process_events(&mut display).await?;
        draw(&mut stdout, &display)?;

        thread::sleep(Duration::from_millis(33));
    }

    execute!(
        stdout,
        Clear(ClearType::All),
        Show,
        EnableBlinking,
        MoveTo(0, 0),
        Print("🦀 Thanks for using AEQ-CAC 🦀"),
        Clear(ClearType::UntilNewLine),
        MoveToNextLine(1)
    )?;
    stdout.flush()?;

    println!();
    Ok(())
}
