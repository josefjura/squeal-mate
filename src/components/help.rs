use tui_popup::Popup;

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, app::AppState, config::Settings, tui::Frame};

pub struct Help<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    visible: bool,
    text: Text<'a>,
}

impl<'a> Help<'a> {
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
            (
                "s".to_string(),
                "Select all after cursor in current directory".to_string(),
            ),
            ("S".to_string(), "Select all after cursor".to_string()),
            (
                "d".to_string(),
                "Select all in current directory".to_string(),
            ),
            ("x".to_string(), "Unselect current file".to_string()),
            ("X".to_string(), "Unselect all in directory".to_string()),
            ("r".to_string(), "Run selected scripts".to_string()),
            (
                "R".to_string(),
                "Run selected scripts, skipping errors".to_string(),
            ),
        ];

        let max = lines.iter().map(|line| line.0.len()).max().unwrap_or(1);

        let text: Text = lines
            .iter()
            .map(|line| Span::raw(format!(" {:>kwidth$} | {} ", line.0, line.1, kwidth = max)))
            .collect();

        Self {
            command_tx: None,
            config: Settings::default(),
            visible: false,
            text,
        }
    }
}

impl<'a> Component for Help<'a> {
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
            Action::ToggleHelp => self.visible = !self.visible,
            Action::CloseHelp => self.visible = false,
            Action::SwitchMode(_) => self.visible = false,
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, _area: Rect, _: &AppState) -> Result<()> {
        if self.visible {
            let popup = Popup::new(self.text.clone())
                .title("Keybindings")
                .style(Style::new().black().on_light_yellow());

            f.render_widget(&popup, f.area());
        }

        Ok(())
    }
}
