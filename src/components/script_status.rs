use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
};
use std::vec;
use tokio::sync::mpsc::UnboundedSender;

use throbber_widgets_tui::ThrobberState;

use super::Component;
use crate::{
    action::Action,
    app::{AppState, Script, ScriptState},
    config::Settings,
    tui::Frame,
};

pub struct ScriptStatus {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    message: String,
    path: String,
    spinner_state: ThrobberState,
}

impl ScriptStatus {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            message: "".into(),
            spinner_state: ThrobberState::default(),
            path: "".into(),
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
                let message = match &result_line {
                    Some(Script {
                        state: ScriptState::Error,
                        error: Some(err),
                        ..
                    }) => err.clone(),
                    Some(Script {
                        state: ScriptState::Finished,
                        elapsed: Some(elapsed),
                        ..
                    }) => format!("Finished in: {}ms", elapsed),
                    None => String::from(""),
                    _ => String::from(""),
                };

                self.message = message;
                self.path = result_line.map_or(String::from(""), |f| f.relative_path)
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, _: &AppState) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(2),
                Constraint::Fill(1), // first row
            ])
            .split(area);

        let text = vec![
            Line::from(Span::raw(&self.path)),
            Line::from(Span::raw(&self.message)),
        ];

        let content = Paragraph::new(text)
            .block(
                Block::new()
                    .title("Status")
                    .title_top("Press h for help")
                    .title_alignment(Alignment::Right)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain)
                    .padding(Padding::horizontal(2)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(content, rects[1]);

        // if self.loading {
        //     let spinner = Throbber::default()
        //         .style(Style::default().fg(Color::Yellow))
        //         .label("Working ")
        //         .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
        //         .use_type(throbber_widgets_tui::WhichUse::Spin);

        //     f.render_stateful_widget(spinner, line[1], &mut self.spinner_state);
        // }

        Ok(())
    }
}
