use async_trait::async_trait;
use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
};
use std::{collections::HashMap, vec};
use tokio::sync::mpsc::UnboundedSender;

use throbber_widgets_tui::{Throbber, ThrobberState};

use super::Component;
use crate::{
    action::Action,
    app::MessageType,
    entries::{ResultLine, ResultState},
    tui::Frame,
};

pub struct ScriptStatus {
    command_tx: Option<UnboundedSender<Action>>,
    config: HashMap<String, String>,
    message: String,
    path: String,
    message_type: MessageType,
    spinner_state: ThrobberState,
    loading: bool,
}

impl ScriptStatus {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: HashMap::<String, String>::default(),
            message: "".into(),
            message_type: MessageType::Info,
            spinner_state: ThrobberState::default(),
            loading: false,
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

    fn register_config_handler(&mut self, config: HashMap<String, String>) -> Result<()> {
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
                        state: ResultState::ERROR,
                        error: Some(err),
                        ..
                    }) => String::from(err.clone()),
                    Some(ResultLine {
                        state: ResultState::FINISHED,
                        elapsed: Some(elapsed),
                        ..
                    }) => elapsed.to_string(),
                    None => String::from(""),
                    _ => String::from(""),
                };

                self.message = message.into();
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
                        state: ResultState::ERROR,
                        error: Some(err),
                        ..
                    }) => String::from(err.clone()),
                    Some(ResultLine {
                        state: ResultState::FINISHED,
                        elapsed: Some(elapsed),
                        ..
                    }) => format!("Finished in: {}ms", elapsed.to_string()),
                    None => String::from(""),
                    _ => String::from(""),
                };

                self.message = message.into();
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