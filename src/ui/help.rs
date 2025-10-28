use tui_popup::Popup;

use color_eyre::eyre::Result;
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, app::AppState, infrastructure::Settings, screen::Mode, tui::Frame};

pub struct Help<'a> {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    visible: bool,
    text: Text<'a>,
    current_mode: Mode,
}

impl<'a> Help<'a> {
    pub fn new() -> Self {
        let current_mode = Mode::FileChooser;
        let text = Self::generate_help_text(current_mode);

        Self {
            command_tx: None,
            config: Settings::default(),
            visible: false,
            text,
            current_mode,
        }
    }

    fn generate_help_text(mode: Mode) -> Text<'static> {
        let mut text_lines = Vec::new();

        // Mode-specific header
        let mode_name = match mode {
            Mode::FileChooser => "FILE BROWSER",
            Mode::ScriptRunner => "SCRIPT EXECUTION",
            Mode::Unified => "UNIFIED VIEW",
        };

        text_lines.push(Line::styled(
            format!("── {} MODE ──", mode_name),
            Style::default().bold().cyan(),
        ));
        text_lines.push(Line::raw(""));

        // Getting Started section
        text_lines.push(Line::styled("── GETTING STARTED ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));

        match mode {
            Mode::FileChooser => {
                text_lines.push(Line::raw("  1. Navigate files/folders with ↑↓ keys"));
                text_lines.push(Line::raw("  2. Enter folders with Enter"));
                text_lines.push(Line::raw("  3. Select scripts with Space"));
                text_lines.push(Line::raw("  4. Press 'r' to run selected scripts"));
                text_lines.push(Line::raw("  5. Press Tab to view execution results"));
            }
            Mode::ScriptRunner => {
                text_lines.push(Line::raw("  1. View script execution output with ↑↓"));
                text_lines.push(Line::raw("  2. Press Tab to return to file browser"));
                text_lines.push(Line::raw("  3. Running scripts show a spinner"));
                text_lines.push(Line::raw("  4. Progress shown as 'X/Y completed'"));
            }
            Mode::Unified => {
                text_lines.push(Line::raw("  1. All panels visible at once (lazygit-style)"));
                text_lines.push(Line::raw("  2. Files on left, details/log on right"));
                text_lines.push(Line::raw("  3. Navigate with ↑↓, select with Space"));
                text_lines.push(Line::raw("  4. Press 'r' to run, Tab to cycle panels"));
                text_lines.push(Line::raw("  5. See real-time execution in log panel"));
            }
        }
        text_lines.push(Line::raw(""));

        // Navigation section
        text_lines.push(Line::styled("── NAVIGATION ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));

        let nav_keys = match mode {
            Mode::FileChooser => vec![
                ("↑ / k", "Move cursor up"),
                ("↓ / j", "Move cursor down"),
                ("Home", "Jump to top of list"),
                ("End", "Jump to bottom of list"),
                ("Enter", "Enter directory"),
                ("Backspace", "Go up one level"),
                ("Tab", "Switch to execution view"),
            ],
            Mode::ScriptRunner => vec![
                ("↑ / k", "Scroll output up"),
                ("↓ / j", "Scroll output down"),
                ("Home", "Jump to first script"),
                ("End", "Jump to last script"),
                ("Tab", "Switch to file browser"),
            ],
            Mode::Unified => vec![
                ("↑ / k", "Move cursor up"),
                ("↓ / j", "Move cursor down"),
                ("Enter", "Enter directory"),
                ("Backspace", "Go up one level"),
                ("Tab", "Cycle focused panel"),
            ],
        };

        for (key, desc) in nav_keys {
            text_lines.push(Line::raw(format!("  {:>12}  {}", key, desc)));
        }
        text_lines.push(Line::raw(""));

        // Mode-specific sections
        match mode {
            Mode::FileChooser => {
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
                    text_lines.push(Line::raw(format!("  {:>12}  {}", key, desc)));
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
                    text_lines.push(Line::raw(format!("  {:>12}  {}", key, desc)));
                }
                text_lines.push(Line::raw(""));
            }
            Mode::ScriptRunner => {
                // Execution info
                text_lines.push(Line::styled("── EXECUTION INFO ──", Style::default().bold().yellow()));
                text_lines.push(Line::raw(""));
                text_lines.push(Line::raw("  Script output is shown in real-time"));
                text_lines.push(Line::raw("  ✓ = Finished successfully"));
                text_lines.push(Line::raw("  ✗ = Error occurred"));
                text_lines.push(Line::raw("  ⟳ = Currently running"));
                text_lines.push(Line::raw(""));
            }
            Mode::Unified => {
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
                    text_lines.push(Line::raw(format!("  {:>12}  {}", key, desc)));
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
                    text_lines.push(Line::raw(format!("  {:>12}  {}", key, desc)));
                }
                text_lines.push(Line::raw(""));

                // Panel info
                text_lines.push(Line::styled("── PANELS ──", Style::default().bold().yellow()));
                text_lines.push(Line::raw(""));
                text_lines.push(Line::raw("  Left: File browser with tree view"));
                text_lines.push(Line::raw("  Right Top: Script details & preview"));
                text_lines.push(Line::raw("  Right Bottom: Real-time execution log"));
                text_lines.push(Line::raw("  Bottom: Persistent command bar"));
                text_lines.push(Line::raw(""));
            }
        }

        // General section (same for both modes)
        text_lines.push(Line::styled("── GENERAL ──", Style::default().bold().yellow()));
        text_lines.push(Line::raw(""));
        text_lines.push(Line::raw(format!("  {:>12}  {}", "? / h", "Toggle this help")));
        text_lines.push(Line::raw(format!("  {:>12}  {}", "q", "Quit application")));

        Text::from(text_lines)
    }

    fn update_help_text(&mut self) {
        self.text = Self::generate_help_text(self.current_mode);
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
            Action::SwitchMode(mode) => {
                self.visible = false;
                self.current_mode = mode;
                self.update_help_text();
            }
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
