use crate::{
    action::{Action, PanelFocus},
    infrastructure::Settings,
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

    /// Get the count of completed scripts (finished or error) and total scripts
    pub fn execution_progress(&self) -> (usize, usize) {
        let total = self.selected.len();
        let completed = self
            .selected
            .iter()
            .filter(|s| matches!(s.state, ScriptState::Finished | ScriptState::Error))
            .count();
        (completed, total)
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
    pub focused_panel: PanelFocus,  // Track which panel is focused in unified view
}

impl App {
    pub fn new(screens: Vec<Screen>, config: Settings) -> Self {
        Self {
            current_screen: Mode::Unified,  // Start with unified view
            exit: false,
            suspend: false,
            frame_rate: 30.0,
            tick_rate: 1.0,
            screens,
            config,
            state: AppState::new(),
            focused_panel: PanelFocus::FileTree,  // Default to file tree
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
                        // General (always available)
                        (_, KeyCode::Char('q')) => action_tx.send(Action::Quit)?,
                        (_, KeyCode::Char('?')) => action_tx.send(Action::ToggleHelp)?,

                        // Navigation - only when FileTree is focused in Unified mode (or any panel in other modes)
                        (Mode::Unified, KeyCode::Up | KeyCode::Char('k')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::CursorUp)?
                        }
                        (Mode::Unified, KeyCode::Down | KeyCode::Char('j')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::CursorDown)?
                        }
                        (Mode::Unified, KeyCode::Home) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::CursorToTop)?
                        }
                        (Mode::Unified, KeyCode::End) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::CursorToBottom)?
                        }
                        (Mode::Unified, KeyCode::Enter) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::DirectoryOpenSelected)?
                        }
                        (Mode::Unified, KeyCode::Right) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::DirectoryExpand)?
                        }
                        (Mode::Unified, KeyCode::Left) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::DirectoryCollapse)?
                        }
                        (Mode::Unified, KeyCode::Char(' ')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::SelectCurrent)?
                        }
                        (Mode::Unified, KeyCode::Char('A')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::UnselectAll)?
                        }
                        (Mode::Unified, KeyCode::Char('x')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::ToggleSkip)?
                        }
                        (Mode::Unified, KeyCode::Char('n')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::JumpToNextNotRun)?
                        }
                        (Mode::Unified, KeyCode::Char('S')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::SelectFromCursorToEnd)?
                        }
                        (Mode::Unified, KeyCode::Char('r')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::ScriptRun(false))?
                        }
                        (Mode::Unified, KeyCode::Char('R')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::ScriptRun(true))?
                        }

                        // In other modes, all navigation/selection is available
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Up | KeyCode::Char('k')) => {
                            action_tx.send(Action::CursorUp)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Down | KeyCode::Char('j')) => {
                            action_tx.send(Action::CursorDown)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Home) => {
                            action_tx.send(Action::CursorToTop)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::End) => {
                            action_tx.send(Action::CursorToBottom)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Enter) => {
                            action_tx.send(Action::DirectoryOpenSelected)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Right) => {
                            action_tx.send(Action::DirectoryExpand)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Left) => {
                            action_tx.send(Action::DirectoryCollapse)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char(' ')) => {
                            action_tx.send(Action::SelectCurrent)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('A')) => {
                            action_tx.send(Action::UnselectAll)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('x')) => {
                            action_tx.send(Action::ToggleSkip)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('n')) => {
                            action_tx.send(Action::JumpToNextNotRun)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('S')) => {
                            action_tx.send(Action::SelectFromCursorToEnd)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('r')) => {
                            action_tx.send(Action::ScriptRun(false))?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('R')) => {
                            action_tx.send(Action::ScriptRun(true))?
                        }

                        // Clear output - available in FileTree and ExecutionLog panels
                        (Mode::Unified, KeyCode::Char('c')) if matches!(self.focused_panel, PanelFocus::FileTree | PanelFocus::ExecutionLog) => {
                            action_tx.send(Action::ClearOutput)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('c')) => {
                            action_tx.send(Action::ClearOutput)?
                        }
                        // Check for changes (CRC check) - available in FileTree
                        (Mode::Unified, KeyCode::Char('C')) if self.focused_panel == PanelFocus::FileTree => {
                            action_tx.send(Action::CheckForChanges)?
                        }
                        (Mode::FileChooser | Mode::ScriptRunner, KeyCode::Char('C')) => {
                            action_tx.send(Action::CheckForChanges)?
                        }
                        (Mode::FileChooser, KeyCode::Tab) => {
                            action_tx.send(Action::SwitchMode(Mode::ScriptRunner))?
                        }
                        (Mode::ScriptRunner, KeyCode::Tab) => {
                            action_tx.send(Action::SwitchMode(Mode::FileChooser))?
                        }
                        (Mode::Unified, KeyCode::Tab) => {
                            action_tx.send(Action::FocusNextPanel)?
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
                    Action::PanelFocusChanged(focus) => self.focused_panel = focus,
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
