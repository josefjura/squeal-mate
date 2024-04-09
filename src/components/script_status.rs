use async_trait::async_trait;
use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Padding, Paragraph, Wrap,
    },
};
use std::vec;
use tokio::sync::mpsc::UnboundedSender;

use throbber_widgets_tui::ThrobberState;

use super::Component;
use crate::{
    action::Action,
    config::Settings,
    entries::{ResultLine, ResultState},
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

#[async_trait]
impl Component for ScriptStatus {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Settings) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                self.spinner_state.calc_next();
            }
            Action::ScriptHighlighted(result_line) => {
                let message = match &result_line {
                    Some(ResultLine {
                        state: ResultState::Error,
                        error: Some(err),
                        ..
                    }) => err.clone(),
                    Some(ResultLine {
                        state: ResultState::Finished,
                        elapsed: Some(elapsed),
                        ..
                    }) => elapsed.to_string(),
                    None => String::from(""),
                    _ => String::from(""),
                };

                self.message = message;
                self.path = result_line.map_or(String::from(""), |f| {
                    f.result
                        .get_full_path()
                        .map(|i| String::from(i.to_str().unwrap_or_default()))
                        .unwrap_or_default()
                })
            }
            _ => {}
        }
        Ok(None)
    }

    async fn update_background(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::ScriptHighlighted(result_line) => {
                let message = match &result_line {
                    Some(ResultLine {
                        state: ResultState::Error,
                        error: Some(err),
                        ..
                    }) => err.clone(),
                    Some(ResultLine {
                        state: ResultState::Finished,
                        elapsed: Some(elapsed),
                        ..
                    }) => format!("Finished in: {}ms", elapsed),
                    None => String::from(""),
                    _ => String::from(""),
                };

                self.message = message;
                self.path = result_line.map_or(String::from(""), |f| {
                    f.result
                        .get_full_path()
                        .map(|i| String::from(i.to_str().unwrap_or_default()))
                        .unwrap_or_default()
                })
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
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
                    .title(
                        Title::from("Press h for help")
                            .position(Position::Bottom)
                            .alignment(Alignment::Right),
                    )
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
