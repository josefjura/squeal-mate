use std::{
    collections::HashMap,
    io::{self, Stdout, Write},
    path::PathBuf,
};

use crossterm::{
    cursor::{self, EnableBlinking, MoveToNextLine},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::Print,
};

pub fn read_line() -> io::Result<String> {
    let mut line = String::new();
    while let Event::Key(KeyEvent { code, .. }) = event::read()? {
        match code {
            KeyCode::Enter => {
                break;
            }
            KeyCode::Char(c) => {
                line.push(c);
            }
            _ => {}
        }
    }

    Ok(line)
}

pub fn read_path(stdout: &mut Stdout) -> String {
    let _ = stdout.flush();

    let result = match read_line() {
        Ok(content) => content,
        Err(_) => String::new(),
    };

    result
}

pub fn read_and_validate_path(stdout: &mut Stdout, config: HashMap<String, String>) -> PathBuf {
    let mut path_wrapped: Option<PathBuf> = None;

    if let Some(content) = config.get("path") {
        return PathBuf::from(content);
    }

    while path_wrapped.is_none() {
        let _ = execute!(
            stdout,
            Print("Path not found, please provide a valid base path."),
            MoveToNextLine(1),
            cursor::Show,
            EnableBlinking
        );
        let test_path = read_path(stdout);
        let candidate = PathBuf::from(test_path);

        if candidate.exists() {
            path_wrapped = Some(candidate)
        };
    }

    path_wrapped.unwrap()
}

pub fn round_up_division(first: usize, second: usize) -> usize {
    return (first + second - 1) / second;
}
