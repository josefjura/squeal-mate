use color_eyre::eyre::Result;
use ratatui::{
    layout::Size,
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::{Action, PanelFocus}, app::AppState, infrastructure::Settings, tui::Frame};

/// Persistent command bar shown at the bottom of the screen
/// Displays context-sensitive keyboard shortcuts
pub struct CommandBar {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    focused_panel: Option<PanelFocus>,  // Track focused panel in unified view
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            focused_panel: None,
        }
    }

    fn get_commands(&self) -> Vec<(&str, &str)> {
        match self.focused_panel {
            Some(PanelFocus::FileTree) | None => {
                // File tree commands (or default when no panel focus)
                vec![
                    ("↑↓/jk", "navigate"),
                    ("n", "next not run"),
                    ("←→", "collapse/expand"),
                    ("enter", "toggle"),
                    ("space", "select"),
                    ("S", "select to end"),
                    ("x", "skip"),
                    ("A", "clear all"),
                    ("r", "run"),
                    ("c", "clear output"),
                    ("C", "check changes"),
                    ("tab", "switch panel"),
                    ("?", "help"),
                    ("q", "quit"),
                ]
            }
            Some(PanelFocus::ScriptPreview) => {
                // Script preview commands (read-only)
                vec![
                    ("tab", "switch panel"),
                    ("?", "help"),
                    ("q", "quit"),
                ]
            }
            Some(PanelFocus::ExecutionLog) => {
                // Execution log commands (could add scrolling later)
                vec![
                    ("c", "clear output"),
                    ("tab", "switch panel"),
                    ("?", "help"),
                    ("q", "quit"),
                ]
            }
        }
    }
}

impl Component for CommandBar {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Settings) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }

    fn handle_events(&mut self, _event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        // Command bar doesn't handle events directly
        Ok(None)
    }

    fn update(&mut self, _state: &mut AppState, action: Action) -> Result<Option<Action>> {
        // Listen for panel focus changes
        match action {
            Action::PanelFocusChanged(focus) => {
                self.focused_panel = Some(focus);
                return Ok(Some(Action::Render));
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, _state: &AppState) -> Result<()> {
        let commands = self.get_commands();

        // Build the command text with colored keys
        let mut spans = Vec::new();

        for (i, (key, desc)) in commands.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" │ "));
            }

            // Key in cyan
            spans.push(Span::styled(*key, Style::default().fg(Color::Cyan).bold()));

            spans.push(Span::raw(":"));

            // Description in normal text
            spans.push(Span::styled(*desc, Style::default().fg(Color::Gray)));
        }

        let text = Line::from(spans);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .alignment(Alignment::Left);

        f.render_widget(paragraph, area);

        Ok(())
    }
}

impl Default for CommandBar {
    fn default() -> Self {
        Self::new()
    }
}
