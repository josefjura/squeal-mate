mod border;
mod config;
mod list;
mod utils;

use border::draw_rect;
use config::{ensure_config_dir, setup_config};
use crossterm::{
    cursor::{DisableBlinking, EnableBlinking, Hide, MoveTo, MoveToNextLine, Show},
    event::{poll, read, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Print, Stylize},
    terminal::{self, Clear, ClearType},
};

use list::{Entry, FileList, Name};
use std::{
    collections::HashMap,
    error::Error,
    fmt::format,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    thread,
    time::Duration,
};
use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

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
enum Screen<'a> {
    FileChooser(FileList, Help<'a>),
    Quit,
}

struct Display<'a> {
    window_width: usize,
    window_height: usize,
    base_path: PathBuf,
    error: Option<String>,
    current_screen: Screen<'a>,
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
                    if let Screen::FileChooser(list, help) = &mut display.current_screen {
                        list.resize(x as usize - help.width as usize, y as usize - 2)
                    }
                }
            }
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
                Screen::FileChooser(list, _) => match key.code {
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

fn draw_error(display: &Display, stdout: &mut io::Stdout) -> Result<(), Box<dyn Error>> {
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

fn draw_selection_help(
    stdout: &mut io::Stdout,
    display: &Display,
    help: &Help,
) -> Result<(), Box<dyn Error>> {
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
) -> Result<(), Box<dyn Error>> {
    draw_list(stdout, list)?;

    draw_selection_help(stdout, display, help)?;

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

fn draw(stdout: &mut io::Stdout, display: &Display) -> Result<(), Box<dyn Error>> {
    if display.error.is_some() {
        draw_error(display, stdout)?;
    } else {
        match &display.current_screen {
            Screen::FileChooser(list, help) => {
                draw_selection(display, list, help, stdout)?;
            }
            _ => (),
        }
    }

    stdout.flush()?;
    Ok(())
}

fn init_tui(stdout: &mut io::Stdout) -> Result<(), Box<dyn std::error::Error>> {
    execute!(stdout, Clear(ClearType::All))?;

    execute!(stdout, Hide, DisableBlinking)?;
    stdout.flush()?;

    Ok(())
}

async fn start_tui(
    stdout: &mut io::Stdout,
    rows: u16,
    cols: u16,
    config: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    terminal::enable_raw_mode()?;
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

    let help = Help::create(&lines, (1, 0), ':');

    let mut display = Display {
        window_height: rows as usize,
        window_width: cols as usize,
        error: None,
        base_path: path,
        current_screen: Screen::FileChooser(
            FileList {
                height: list_height,
                width: (cols - help.width - 1) as usize,
                page_index: 0,
                cursor: 0,
                entries,
            },
            help,
        ),
    };

    while display.current_screen != Screen::Quit {
        process_events(&mut display).await?;
        draw(stdout, &display)?;

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

    return Ok(());
}

fn draw_help(stdout: &mut io::Stdout) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = ensure_config_dir()?;
    let config_path_str = config_path.to_str().expect("Unknown host system");
    let version = env!("CARGO_PKG_VERSION");
    let version_msg = format!("Version: {}\n", version);
    let config_msg = format!("Config src: {}\n", config_path_str);

    execute!(
        stdout,
        Print("🦀 Aequitas Command And Control Console 🦀\n"),
        Print("\n"),
        Print(version_msg),
				Print("Edition: Ultimate\n"),
        Print(config_msg)
    )?;

    stdout.flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _clean_up = CleanUp;
    let mut stdout = io::stdout();

    let config = setup_config().expect("Error while loading config!");

    let (cols, rows) = terminal::size()?;

    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, command] if (command.as_str() == "help") => draw_help(&mut stdout)?,
        _ => {
            init_tui(&mut stdout)?;
            start_tui(&mut stdout, rows, cols, &config).await?;
        }
    }

    println!();
    Ok(())
}
