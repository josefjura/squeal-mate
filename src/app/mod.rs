use ratatui::style::Stylize;

use crate::{Database, Entry, ListState};

use std::{error, fmt::Display, path::PathBuf};

mod methods;

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub enum Message {
    Success(String),
    Error(String),
    Info(String),
}

pub struct UiState {
    pub list: ListState,
}

pub struct App {
    pub base_path: PathBuf,
    pub current_screen: Screen,
    pub message: Option<Message>,
    pub exit: bool,
    pub connection: Database,
    pub ui_state: UiState,
}

#[derive(PartialEq)]
pub enum Screen {
    FileChooser { entries: Vec<Entry> },
}

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Error(text) => write!(f, "{}", text.clone().red()),
            Message::Success(text) => write!(f, "{}", text.clone().green()),
            Message::Info(text) => write!(f, "{}", text.clone().white()),
        }
    }
}
