use crate::{
    action::Action,
    config::Settings,
    screen::{Mode, Screen},
    tui,
};

use color_eyre::eyre;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::Rect;
use tokio::sync::mpsc;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum ScriptState {
    Finished,
    Running,
    Error,
    None,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct Script {
    pub relative_path: String,
    pub state: ScriptState,
    pub error: Option<String>,
    pub elapsed: Option<u128>,
}

impl Script {
    pub fn none(path: &str) -> Self {
        Self {
            error: None,
            relative_path: path.into(),
            state: ScriptState::None,
            elapsed: None,
        }
    }

    #[allow(dead_code)]
    pub fn error(path: &str, error: String) -> Self {
        Self {
            error: Some(error),
            relative_path: path.into(),
            state: ScriptState::Error,
            elapsed: None,
        }
    }

    #[allow(dead_code)]
    pub fn finished(path: &str, elapsed: u128) -> Self {
        Self {
            error: None,
            relative_path: path.into(),
            state: ScriptState::Finished,
            elapsed: Some(elapsed),
        }
    }
}

pub struct AppState {
    pub selected: Vec<Script>,
}

impl AppState {
    pub fn new() -> Self {
        Self { selected: vec![] }
    }

    pub fn add(&mut self, script: String) {
        if !self.selected.iter().any(|s| s.relative_path == script) {
            self.selected.push(Script::none(&script));
            self.selected.sort()
        }
    }

    pub fn remove(&mut self, script: String) {
        self.selected.retain(|s| s.relative_path != script);
        self.selected.sort()
    }

    pub fn remove_many(&mut self, script: &[String]) {
        self.selected.retain(|s| !script.contains(&s.relative_path));
        self.selected.sort()
    }

    pub fn toggle(&mut self, scripts: String) {
        if self.selected.iter().any(|s| s.relative_path == scripts) {
            self.selected.retain(|s| s.relative_path != scripts);
        } else {
            self.add(scripts);
        }
        self.selected.sort()
    }

    pub fn toggle_many(&mut self, scripts: &[String]) {
        if self
            .selected
            .iter()
            .any(|s| scripts.contains(&s.relative_path))
        {
            self.selected
                .retain(|s| !scripts.contains(&s.relative_path));
        } else {
            self.add_many(scripts);
        }

        self.selected.sort()
    }

    pub fn add_many(&mut self, scripts: &[String]) {
        let new_items: Vec<Script> = scripts
            .iter()
            .filter(|s| !self.selected.iter().any(|r| r.relative_path == **s))
            .map(|s| Script::none(s))
            .collect();

        self.selected.extend(new_items);
        self.selected.sort()
    }
}

pub struct App {
    pub current_screen: Mode,
    pub exit: bool,
    pub suspend: bool,
    pub tick_rate: f64,
    pub frame_rate: f64,
    pub screens: Vec<Screen>,
    pub config: Settings,
    pub state: AppState,
}

impl App {
    pub fn new(screens: Vec<Screen>, config: Settings) -> Self {
        Self {
            current_screen: Mode::FileChooser,
            exit: false,
            suspend: false,
            frame_rate: 30.0,
            tick_rate: 1.0,
            screens,
            config,
            state: AppState::new(),
        }
    }

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
                    tui::Event::Key(key) => match (self.current_screen, key.code) {
                        (_, KeyCode::Char('z')) if key.modifiers == KeyModifiers::CONTROL => {
                            action_tx.send(Action::Suspend)?
                        }
                        (_, KeyCode::Char('c')) if key.modifiers == KeyModifiers::CONTROL => {
                            action_tx.send(Action::Quit)?
                        }
                        (_, KeyCode::Char('q')) => action_tx.send(Action::Quit)?,
                        (_, KeyCode::Char('r')) => action_tx.send(Action::ScriptRun(false))?,
                        (_, KeyCode::Char('R')) => action_tx.send(Action::ScriptRun(true))?,
                        (_, KeyCode::Char(' ')) => action_tx.send(Action::SelectCurrent)?,
                        (_, KeyCode::Char('s')) => {
                            action_tx.send(Action::SelectAllAfterInDirectory)?
                        }
                        (_, KeyCode::Char('S')) => action_tx.send(Action::SelectAllAfter)?,
                        (_, KeyCode::Char('d')) => action_tx.send(Action::SelectAllInDirectory)?,
                        (_, KeyCode::Char('x')) => action_tx.send(Action::UnselectCurrent)?,
                        (_, KeyCode::Char('X')) => action_tx.send(Action::UnselectAll)?,
                        (_, KeyCode::Char('h')) => action_tx.send(Action::ToggleHelp)?,
                        (_, KeyCode::Up) => action_tx.send(Action::CursorUp)?,
                        (_, KeyCode::Down) => action_tx.send(Action::CursorDown)?,
                        (_, KeyCode::Home) => action_tx.send(Action::CursorToTop)?,
                        (_, KeyCode::End) => action_tx.send(Action::CursorToBottom)?,
                        (_, KeyCode::Enter) => action_tx.send(Action::DirectoryOpenSelected)?,
                        (_, KeyCode::Backspace) => action_tx.send(Action::DirectoryLeave)?,
                        (Mode::FileChooser, KeyCode::Tab) => {
                            action_tx.send(Action::SwitchMode(Mode::ScriptRunner))?
                        }
                        (Mode::ScriptRunner, KeyCode::Tab) => {
                            action_tx.send(Action::SwitchMode(Mode::FileChooser))?
                        }
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
                    //log::debug!("{action:?}");
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
                                    let r = component.draw(f, f.area(), &self.state);
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
                                    let r = component.draw(f, f.area(), &self.state);
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

                if let Action::EntryStatusChanged(_, _) = action {
                    for screen in self.screens.iter_mut() {
                        for component in screen.components.iter_mut() {
                            if action != Action::Tick && action != Action::Render {
                                log::debug!("Running: {action:?} {:?}", screen.mode);
                            }
                            if let Some(action) =
                                component.update(&mut self.state, action.clone())?
                            {
                                action_tx.send(action)?
                            };
                        }
                    }
                } else {
                    let screen = self
                        .screens
                        .iter_mut()
                        .find(|f| f.mode == self.current_screen);

                    if let Some(screen) = screen {
                        for component in screen.components.iter_mut() {
                            if action != Action::Tick && action != Action::Render {
                                log::debug!("Running: {action:?} {:?}", screen.mode);
                            }
                            if let Some(action) =
                                component.update(&mut self.state, action.clone())?
                            {
                                action_tx.send(action)?
                            };
                        }
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
