use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
    layout::Size,
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    app::{AppState, Script},
    infrastructure::Settings,
    tui::Frame,
};

/// Script preview panel that shows details and preview of the selected script
pub struct ScriptPreview {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    highlighted_script: Option<Script>,
}

impl ScriptPreview {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            highlighted_script: None,
        }
    }

    fn render_placeholder(&self, f: &mut Frame<'_>, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No script selected",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Select a script from the file tree",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "to see its details here.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    fn render_script_details(&self, f: &mut Frame<'_>, area: Rect, script: &Script) {
        // TODO: Load actual file content and metadata
        // For now, show basic info

        let status_text = match script.state {
            crate::app::ScriptState::Finished => ("✓ Success", Color::Green),
            crate::app::ScriptState::Error => ("✗ Error", Color::Red),
            crate::app::ScriptState::Running => ("⟳ Running", Color::Yellow),
            crate::app::ScriptState::None => ("• New", Color::Cyan),
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&script.relative_path, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(status_text.0, Style::default().fg(status_text.1).bold()),
            ]),
        ];

        if let Some(elapsed) = script.elapsed {
            lines.push(Line::from(vec![
                Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.2}s", elapsed as f64 / 1000.0),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if let Some(ref error) = script.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red).bold()),
            ]));
            lines.push(Line::from(""));

            // Show first few lines of error
            for (i, line) in error.lines().take(10).enumerate() {
                if i >= 10 {
                    lines.push(Line::from(Span::styled(
                        "... (truncated)",
                        Style::default().fg(Color::DarkGray).italic(),
                    )));
                    break;
                }
                lines.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(Color::Red),
                )));
            }
        } else {
            // Show placeholder for file preview
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "─".repeat(30),
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "File preview",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "─".repeat(30),
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "TODO: Load and display first 20 lines of SQL file",
                Style::default().fg(Color::DarkGray).italic(),
            )));
        }

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .scroll((0, 0));

        f.render_widget(paragraph, area);
    }
}

impl Component for ScriptPreview {
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
        // Script preview doesn't handle events (not focusable yet)
        Ok(None)
    }

    fn update(&mut self, _state: &mut AppState, action: Action) -> Result<Option<Action>> {
        // Listen for highlighted script changes
        match action {
            Action::ScriptHighlighted(script) => {
                self.highlighted_script = script;
                Ok(Some(Action::Render))
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, _state: &AppState) -> Result<()> {
        if let Some(ref script) = self.highlighted_script {
            self.render_script_details(f, area, script);
        } else {
            self.render_placeholder(f, area);
        }

        Ok(())
    }
}

impl Default for ScriptPreview {
    fn default() -> Self {
        Self::new()
    }
}
