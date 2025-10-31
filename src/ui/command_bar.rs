use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    layout::Size,
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    app::AppState,
    infrastructure::Settings,
    tui::Frame,
};

/// Persistent command bar shown at the bottom of the screen
/// Displays context-sensitive keyboard shortcuts
pub struct CommandBar {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
        }
    }

    fn get_commands(&self) -> Vec<(&str, &str)> {
        vec![
            ("↑↓", "navigate"),
            ("enter", "expand/collapse"),
            ("space", "select"),
            ("r", "run"),
            ("R", "run all"),
            ("x", "clear"),
            ("tab", "next panel"),
            ("?", "help"),
            ("q", "quit"),
        ]
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

    fn update(&mut self, _state: &mut AppState, _action: Action) -> Result<Option<Action>> {
        // Command bar doesn't need to update
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
            spans.push(Span::styled(
                *key,
                Style::default().fg(Color::Cyan).bold(),
            ));

            spans.push(Span::raw(":"));

            // Description in normal text
            spans.push(Span::styled(
                *desc,
                Style::default().fg(Color::Gray),
            ));
        }

        let text = Line::from(spans);
        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
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
