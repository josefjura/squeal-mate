use crate::{
    action::{Action, PanelFocus},
    infrastructure::Settings,
    keymap::key_to_action,
    tui::{self, Frame},
    ui::Component,
};

use color_eyre::eyre;
use ratatui::{
    prelude::Rect,
    style::{Color, Style},
    widgets::{Clear, Paragraph},
};
use tokio::sync::mpsc::{self, UnboundedSender};

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

type Root = Box<dyn Component + Send + Sync>;

/// Draws the root component, then overlays the current error banner (if any).
/// A draw failure is reported as `Action::Error`; a closed channel is ignored
/// because panicking inside the draw closure would leave the terminal broken.
fn draw_frame(
    root: &mut (dyn Component + Send + Sync),
    state: &AppState,
    error: Option<&str>,
    action_tx: &UnboundedSender<Action>,
    f: &mut Frame<'_>,
) {
    if let Err(e) = root.draw(f, f.area(), state) {
        let _ = action_tx.send(Action::Error(format!("Failed to draw: {:?}", e)));
    }

    if let Some(message) = error {
        let area = f.area();
        if area.height == 0 {
            return;
        }
        let banner = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        f.render_widget(Clear, banner);
        f.render_widget(
            Paragraph::new(message).style(Style::default().fg(Color::White).bg(Color::Red)),
            banner,
        );
    }
}

pub struct App {
    pub exit: bool,
    pub suspend: bool,
    pub tick_rate: f64,
    pub frame_rate: f64,
    pub root: Root,
    pub config: Settings,
    pub state: AppState,
    pub focused_panel: PanelFocus,
    /// Last error reported via `Action::Error`, shown until a key is pressed.
    pub error: Option<String>,
}

impl App {
    pub fn new(root: Root, config: Settings) -> Self {
        Self {
            exit: false,
            suspend: false,
            frame_rate: 30.0,
            tick_rate: 1.0,
            root,
            config,
            state: AppState::new(),
            focused_panel: PanelFocus::FileTree,
            error: None,
        }
    }

    fn draw(
        &mut self,
        tui: &mut tui::Tui,
        action_tx: &UnboundedSender<Action>,
    ) -> eyre::Result<()> {
        tui.draw(|f| {
            draw_frame(
                self.root.as_mut(),
                &self.state,
                self.error.as_deref(),
                action_tx,
                f,
            )
        })?;
        Ok(())
    }

    pub async fn run(&mut self) -> eyre::Result<()> {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();

        let mut tui = tui::Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        // tui.mouse(true);
        tui.enter()?;

        self.root.register_action_handler(action_tx.clone())?;
        self.root.register_config_handler(self.config.clone())?;
        self.root.init(tui.size()?)?;

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    tui::Event::Quit => action_tx.send(Action::Quit)?,
                    tui::Event::Tick => action_tx.send(Action::Tick)?,
                    tui::Event::Render => action_tx.send(Action::Render)?,
                    tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                    tui::Event::Key(key) => {
                        self.error = None;
                        if let Some(action) =
                            key_to_action(self.focused_panel, key.code, key.modifiers)
                        {
                            action_tx.send(action)?
                        }
                    }
                    _ => {}
                }

                if let Some(action) = self.root.handle_events(Some(e.clone()))? {
                    action_tx.send(action)?;
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                match action {
                    Action::Quit => self.exit = true,
                    Action::Suspend => self.suspend = true,
                    Action::Resume => self.suspend = false,
                    Action::PanelFocusChanged(focus) => self.focused_panel = focus,
                    Action::Error(ref message) => self.error = Some(message.clone()),
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;
                        self.draw(&mut tui, &action_tx)?;
                    }
                    Action::Render => self.draw(&mut tui, &action_tx)?,
                    _ => {}
                }

                if action != Action::Tick && action != Action::Render {
                    log::debug!("Running: {action:?}");
                }
                if let Some(action) = self.root.update(&mut self.state, action)? {
                    action_tx.send(action)?
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    struct FailingRoot;

    impl Component for FailingRoot {
        fn draw(&mut self, _: &mut Frame<'_>, _: Rect, _: &AppState) -> eyre::Result<()> {
            Err(eyre::eyre!("boom"))
        }
    }

    struct BlankRoot;

    impl Component for BlankRoot {
        fn draw(&mut self, _: &mut Frame<'_>, _: Rect, _: &AppState) -> eyre::Result<()> {
            Ok(())
        }
    }

    fn render(
        root: &mut (dyn Component + Send + Sync),
        error: Option<&str>,
        tx: &UnboundedSender<Action>,
    ) -> String {
        let mut terminal = Terminal::new(TestBackend::new(40, 3)).unwrap();
        terminal
            .draw(|f| draw_frame(root, &AppState::new(), error, tx, f))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..3)
            .map(|y| (0..40).map(|x| buffer[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn draw_failure_is_reported_as_error_action() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        render(&mut FailingRoot, None, &tx);
        match rx.try_recv() {
            Ok(Action::Error(msg)) => assert!(msg.contains("boom")),
            other => panic!("expected Action::Error, got {other:?}"),
        }
    }

    #[test]
    fn draw_failure_with_closed_channel_does_not_panic() {
        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx);
        render(&mut FailingRoot, None, &tx);
    }

    #[test]
    fn error_banner_is_shown_on_last_line() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let screen = render(&mut BlankRoot, Some("Failed to draw: boom"), &tx);
        assert!(screen.lines().last().unwrap().contains("Failed to draw: boom"));
    }

    #[test]
    fn no_banner_without_error() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let screen = render(&mut BlankRoot, None, &tx);
        assert!(screen.trim().is_empty());
    }
}
