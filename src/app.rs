use crate::{action::Action, components::Component, tui};
use color_eyre::eyre;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::Rect;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Success,
    Error,
    Info,
}

pub struct Screen {
    pub mode: Mode,
    pub components: Vec<Box<dyn Component>>,
}

impl Screen {
    pub fn new(mode: Mode, components: Vec<Box<dyn Component>>) -> Self {
        Self { mode, components }
    }
}

pub struct App {
    pub current_screen: Mode,
    pub exit: bool,
    pub suspend: bool,
    pub tick_rate: f64,
    pub frame_rate: f64,
    pub screens: Vec<Screen>,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    FileChooser,
    ScriptRunner,
}

impl App {
    pub async fn run(&mut self) -> eyre::Result<()> {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();

        let mut tui = tui::Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        // tui.mouse(true);
        tui.enter()?;

        for screen in self.screens.iter_mut() {
            for component in screen.components.iter_mut() {
                component.register_action_handler(action_tx.clone())?;
            }
        }

        for screen in self.screens.iter_mut() {
            for component in screen.components.iter_mut() {
                component.register_config_handler(self.config.clone())?;
            }
        }

        for screen in self.screens.iter_mut() {
            for component in screen.components.iter_mut() {
                component.init(tui.size()?)?;
            }
        }

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    tui::Event::Quit => action_tx.send(Action::Quit)?,
                    tui::Event::Tick => action_tx.send(Action::Tick)?,
                    tui::Event::Render => action_tx.send(Action::Render)?,
                    tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                    tui::Event::SwitchMode(mode) => action_tx.send(Action::SwitchMode(mode))?,
                    tui::Event::Key(key) => match key.code {
                        KeyCode::Char('z') if key.modifiers == KeyModifiers::CONTROL => {
                            action_tx.send(Action::Suspend)?
                        }
                        KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                            action_tx.send(Action::Quit)?
                        }
                        KeyCode::Char('q') => action_tx.send(Action::Quit)?,
                        KeyCode::Char('s') => action_tx.send(Action::ScriptRun)?,
                        KeyCode::Char('o') => {
                            action_tx.send(Action::SwitchMode(Mode::ScriptRunner))?
                        }
                        KeyCode::Char('O') => {
                            action_tx.send(Action::SwitchMode(Mode::FileChooser))?
                        }
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
                for screen in self.screens.iter_mut() {
                    for component in screen.components.iter_mut() {
                        if let Some(action) = component.handle_events(Some(e.clone()))? {
                            action_tx.send(action)?;
                        }
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
                    Action::SwitchMode(mode) => self.current_screen = mode,
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;
                        let screen = self
                            .screens
                            .iter_mut()
                            .find(|f| f.mode == self.current_screen);
                        if let Some(screen) = screen {
                            tui.draw(|f| {
                                for component in screen.components.iter_mut() {
                                    let r = component.draw(f, f.size());
                                    if let Err(e) = r {
                                        action_tx
                                            .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                            .unwrap();
                                    }
                                }
                            })?;
                        }
                    }
                    Action::Render => {
                        let screen = self
                            .screens
                            .iter_mut()
                            .find(|f| f.mode == self.current_screen);
                        if let Some(screen) = screen {
                            tui.draw(|f| {
                                for component in screen.components.iter_mut() {
                                    let r = component.draw(f, f.size());
                                    if let Err(e) = r {
                                        action_tx
                                            .send(Action::Error(format!("Failed to draw: {:?}", e)))
                                            .unwrap();
                                    }
                                }
                            })?;
                        }
                    }
                    _ => {}
                }
                let screen = self
                    .screens
                    .iter_mut()
                    .find(|f| f.mode == self.current_screen);
                if let Some(screen) = screen {
                    for component in screen.components.iter_mut() {
                        if let Some(action) = component.update(action.clone())? {
                            action_tx.send(action)?
                        };
                    }
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
