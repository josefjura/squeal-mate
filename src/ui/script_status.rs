use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
};
use std::vec;
use tokio::sync::mpsc::UnboundedSender;

use throbber_widgets_tui::{Throbber, ThrobberState};

use super::Component;
use crate::{
    action::Action,
    app::{AppState, Script, ScriptState},
    infrastructure::Settings,
    tui::Frame,
};

pub struct ScriptStatus {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    message: String,
    path: String,
    spinner_state: ThrobberState,
    is_running: bool,
}

impl ScriptStatus {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            message: "".into(),
            spinner_state: ThrobberState::default(),
            path: "".into(),
            is_running: false,
        }
    }
}

impl Component for ScriptStatus {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Settings) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, _: &mut AppState, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                self.spinner_state.calc_next();
            }
            Action::ScriptHighlighted(result_line) => {
                let (message, running) = match &result_line {
                    Some(Script {
                        state: ScriptState::Running,
                        ..
                    }) => (String::from("Running..."), true),
                    Some(Script {
                        state: ScriptState::Error,
                        error: Some(err),
                        ..
                    }) => (err.clone(), false),
                    Some(Script {
                        state: ScriptState::Finished,
                        elapsed: Some(elapsed),
                        ..
                    }) => (format!("Finished in: {}ms", elapsed), false),
                    None => (String::from(""), false),
                    _ => (String::from(""), false),
                };

                self.message = message;
                self.is_running = running;
                self.path = result_line.map_or(String::from(""), |f| f.relative_path)
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &AppState) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(2),
                Constraint::Fill(1), // first row
            ])
            .split(area);

        // Get execution progress
        let (completed, total) = state.execution_progress();

        // Build title with progress if there are selected scripts
        let title = if total > 0 {
            format!("Status - {}/{} completed", completed, total)
        } else {
            String::from("Status")
        };

        // Create content block
        let block = Block::new()
            .title(title)
            .title_top("Press h for help")
            .title_alignment(Alignment::Right)
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .padding(Padding::horizontal(2));

        let inner_area = block.inner(rects[1]);
        f.render_widget(block, rects[1]);

        // Split inner area for text and spinner
        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(12), // space for spinner
            ])
            .split(inner_area);

        let text = vec![
            Line::from(Span::raw(&self.path)),
            Line::from(Span::raw(&self.message)),
        ];

        let content = Paragraph::new(text).wrap(Wrap { trim: false });

        f.render_widget(content, inner_layout[0]);

        // Render spinner when running
        if self.is_running {
            let spinner = Throbber::default()
                .style(Style::default().fg(Color::Yellow))
                .label("Working ")
                .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
                .use_type(throbber_widgets_tui::WhichUse::Spin);

            f.render_stateful_widget(spinner, inner_layout[1], &mut self.spinner_state);
        }

        Ok(())
    }
}
