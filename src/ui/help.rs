use tui_popup::Popup;

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, app::AppState, infrastructure::Settings, tui::Frame};

pub struct Help<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    visible: bool,
    text: Text<'a>,
}

impl<'a> Help<'a> {
    pub fn new() -> Self {
        let mut text_lines = Vec::new();

        // Getting Started section
        text_lines.push(Line::styled("── GETTING STARTED ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));
        text_lines.push(Line::raw("  1. Navigate files with ↑↓ keys"));
        text_lines.push(Line::raw("  2. Select scripts with Space"));
        text_lines.push(Line::raw("  3. Press 'r' to run selected scripts"));
        text_lines.push(Line::raw("  4. Press Tab to view execution results"));
        text_lines.push(Line::raw(""));

        // Navigation section
        text_lines.push(Line::styled("── NAVIGATION ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));

        let nav_keys = vec![
            ("↑↓", "Move up and down"),
            ("Home", "Jump to top of list"),
            ("End", "Jump to bottom of list"),
            ("Enter", "Enter directory"),
            ("Backspace", "Go up one level"),
            ("Tab", "Switch between file/execution screens"),
        ];

        for (key, desc) in nav_keys {
            text_lines.push(Line::raw(format!("  {:>10}  {}", key, desc)));
        }
        text_lines.push(Line::raw(""));

        // Selection section
        text_lines.push(Line::styled("── SELECTION ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));

        let sel_keys = vec![
            ("Space", "Toggle current file/directory"),
            ("d", "Select all in current directory"),
            ("s", "Select all after cursor (current dir)"),
            ("S", "Select all after cursor (recursive)"),
            ("x", "Unselect current file"),
            ("X", "Unselect all"),
        ];

        for (key, desc) in sel_keys {
            text_lines.push(Line::raw(format!("  {:>10}  {}", key, desc)));
        }
        text_lines.push(Line::raw(""));

        // Execution section
        text_lines.push(Line::styled("── EXECUTION ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));

        let exec_keys = vec![
            ("r", "Run selected scripts (stop on error)"),
            ("R", "Run selected scripts (skip errors)"),
        ];

        for (key, desc) in exec_keys {
            text_lines.push(Line::raw(format!("  {:>10}  {}", key, desc)));
        }
        text_lines.push(Line::raw(""));

        // General section
        text_lines.push(Line::styled("── GENERAL ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));
        text_lines.push(Line::raw(format!("  {:>10}  {}", "?", "Toggle this help")));
        text_lines.push(Line::raw(format!("  {:>10}  {}", "q", "Quit application")));

        let text = Text::from(text_lines);

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
