use crate::{action::Action, components::Component, db::Database, tui};
use color_eyre::eyre;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{prelude::Rect, style::Stylize, widgets::ListState};
use std::{collections::HashMap, fmt::Display};
use tokio::sync::mpsc;

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
    pub current_screen: Mode,
    pub message: Option<Message>,
    pub exit: bool,
    pub suspend: bool,
    pub connection: Database,
    pub ui_state: UiState,
    pub tick_rate: f64,
    pub frame_rate: f64,
    pub components: Vec<Box<dyn Component>>,
    pub config: HashMap<String, String>,
}

#[derive(PartialEq)]
pub enum Mode {
    FileChooser,
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

impl App {
    pub async fn run(&mut self) -> eyre::Result<()> {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();

        let mut tui = tui::Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        // tui.mouse(true);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(action_tx.clone())?;
        }

        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }

        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    tui::Event::Quit => action_tx.send(Action::Quit)?,
                    tui::Event::Tick => action_tx.send(Action::Tick)?,
                    tui::Event::Render => action_tx.send(Action::Render)?,
                    tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                    tui::Event::Key(key) => match key.code {
                        KeyCode::Char('z') if key.modifiers == KeyModifiers::CONTROL => {
                            action_tx.send(Action::Suspend)?
                        }
                        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                            action_tx.send(Action::Quit)?
                        }
                        KeyCode::Char('q') => action_tx.send(Action::Quit)?,
                        KeyCode::Up => action_tx.send(Action::CursorUp)?,
                        KeyCode::Down => action_tx.send(Action::CursorDown)?,
                        KeyCode::Home => action_tx.send(Action::CursorToTop)?,
                        KeyCode::End => action_tx.send(Action::CursorToBottom)?,
                        KeyCode::Enter => action_tx.send(Action::DirectoryOpenSelected)?,
                        KeyCode::Backspace => action_tx.send(Action::DirectoryLeave)?,
                        _ => {}
                    },
                    _ => {}
                }
                for component in self.components.iter_mut() {
                    if let Some(action) = component.handle_events(Some(e.clone()))? {
                        action_tx.send(action)?;
                    }
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                if action != Action::Tick && action != Action::Render {
                    log::debug!("{action:?}");
                }
                match action {
                    Action::Tick => {
                        //self.last_tick_key_events.drain(..);
                    }
                    Action::Quit => self.exit = true,
                    Action::Suspend => self.suspend = true,
                    Action::Resume => self.suspend = false,
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;
                        tui.draw(|f| {
                            for component in self.components.iter_mut() {
                                let r = component.draw(f, f.size());
                                if let Err(e) = r {
                                    action_tx
                                        .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                        .unwrap();
                                }
                            }
                        })?;
                    }
                    Action::Render => {
                        tui.draw(|f| {
                            for component in self.components.iter_mut() {
                                let r = component.draw(f, f.size());
                                if let Err(e) = r {
                                    action_tx
                                        .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                        .unwrap();
                                }
                            }
                        })?;
                    }
                    _ => {}
                }
                for component in self.components.iter_mut() {
                    if let Some(action) = component.update(action.clone())? {
                        action_tx.send(action)?
                    };
                }
            }
            if self.suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                tui = tui::Tui::new()?
                    .tick_rate(self.tick_rate)
                    .frame_rate(self.frame_rate);
                // tui.mouse(true);
                tui.enter()?;
            } else if self.exit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }
}
