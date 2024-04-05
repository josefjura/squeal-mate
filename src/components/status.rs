use async_trait::async_trait;
use std::{collections::HashMap, vec};

use chrono::Local;
use color_eyre::eyre::Result;
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use throbber_widgets_tui::{Throbber, ThrobberState};

use super::Component;
use crate::{action::Action, app::MessageType, tui::Frame};

pub struct Status {
    command_tx: Option<UnboundedSender<Action>>,
    config: HashMap<String, String>,
    message: String,
    message_type: MessageType,
    message_timestamp: Option<String>,
    spinner_state: ThrobberState,
    loading: bool,
}

impl Status {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: HashMap::<String, String>::default(),
            message: "".into(),
            message_timestamp: None,
            message_type: MessageType::Info,
            spinner_state: ThrobberState::default(),
            loading: false,
        }
    }
}

#[async_trait]
impl Component for Status {
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
            _ => {}
        }
        Ok(None)
    }

    async fn update_background(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Message(text, m_type) => {
                self.message = text;
                self.message_type = m_type;
                self.message_timestamp = Some(Local::now().format("%H:%M:%S").to_string());
                return Ok(None);
            }
            Action::StartSpinner => {
                self.loading = true;
                return Ok(None);
            }
            Action::StopSpinner => {
                self.loading = false;
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(1), // first row
                Constraint::Fill(1),
                Constraint::Length(1), // first row
            ])
            .split(area);

        let line = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Min(0), Constraint::Length(12)])
            .split(rects[2]);

        let prompt = Span::raw("AEQ-CAC > ").style(Style::default().fg(Color::Red));
        let message_text = self.message.clone();
        let message = if let Some(timestamp) = &self.message_timestamp {
            format!("{} {}", timestamp, message_text)
        } else {
            message_text
        };
        let message_style = match self.message_type {
            MessageType::Info => Style::default(),
            MessageType::Success => Style::default().fg(Color::Green),
            MessageType::Error => Style::default().fg(Color::Red),
        };
        let message_prompt = Span::raw(message).style(message_style);
        let line_draw = Line::default().spans(vec![prompt, message_prompt]);
        f.render_widget(line_draw, line[0]);

        if self.loading {
            let spinner = Throbber::default()
                .style(Style::default().fg(Color::Yellow))
                .label("Working ")
                .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
                .use_type(throbber_widgets_tui::WhichUse::Spin);

            f.render_stateful_widget(spinner, line[1], &mut self.spinner_state);
        }

        Ok(())
    }
}
