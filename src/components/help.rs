use async_trait::async_trait;
use std::{collections::HashMap, vec};
use tui_popup::Popup;

use chrono::Local;
use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;

use throbber_widgets_tui::{Throbber, ThrobberState};

use super::Component;
use crate::{action::Action, app::MessageType, config::Settings, tui::Frame};

pub struct Help {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    visible: bool,
    text: String,
}

impl Help {
    pub fn new() -> Self {
        let lines = vec![
            ("q".to_string(), "Quit".to_string()),
            ("Tab".to_string(), "Switch screen".to_string()),
            (
                "\u{02191}\u{02193}".to_string(),
                "Move up and down".to_string(),
            ),
            ("Home".to_string(), "Top of the list".to_string()),
            ("End".to_string(), "Bottom of the list".to_string()),
            ("Enter".to_string(), "Enter directory".to_string()),
            ("Backspace".to_string(), "Up a level".to_string()),
            ("Space".to_string(), "Toggle file selection".to_string()),
            ("s".to_string(), "Select all in directory".to_string()),
            ("x".to_string(), "Unselect current file".to_string()),
            ("X".to_string(), "Unselect all in directory".to_string()),
            ("r".to_string(), "Run selected scripts".to_string()),
        ];

        let key_length = lines.iter().map(|tuple| tuple.0.len()).max();
        let value_length = lines.iter().map(|tuple| tuple.1.len()).max();

        let text = lines
            .iter()
            .map(|tuple| {
                format!(
                    "{:>kwidth$} | {:vwidth$}",
                    tuple.0,
                    tuple.1,
                    kwidth = key_length.unwrap_or(1),
                    vwidth = value_length.unwrap_or(1)
                )
            })
            .reduce(|acc, line| {
                return acc + "\n" + &line;
            })
            .unwrap_or_default();

        Self {
            command_tx: None,
            config: Settings::default(),
            visible: false,
            text,
        }
    }
}

#[async_trait]
impl Component for Help {
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
            Action::ToggleHelp => self.visible = !self.visible,
            Action::CloseHelp => self.visible = false,
            Action::SwitchMode(_) => self.visible = false,
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        if self.visible {
            let popup = Popup::new("Keybindings", self.text.clone())
                .style(Style::new().black().on_light_yellow());
            f.render_widget(popup.to_widget(), f.size());
        }

        Ok(())
    }
}
