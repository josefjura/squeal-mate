use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{List, ListItem},
    layout::Size,
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    app::AppState,
    infrastructure::Settings,
    tui::Frame,
};

/// Execution log panel that shows real-time script execution progress
pub struct ExecutionLog {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    log_entries: Vec<LogEntry>,
}

#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    message: String,
    level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogLevel {
    Info,
    Success,
    Error,
    Running,
}

impl ExecutionLog {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            log_entries: Vec::new(),
        }
    }

    fn add_log(&mut self, message: String, level: LogLevel) {
        // Format timestamp
        let now = chrono::Local::now();
        let timestamp = now.format("%H:%M:%S").to_string();

        self.log_entries.push(LogEntry {
            timestamp,
            message,
            level,
        });

        // Keep only last 100 entries
        if self.log_entries.len() > 100 {
            self.log_entries.remove(0);
        }
    }

    fn render_logs(&self, f: &mut Frame<'_>, area: Rect) {
        if self.log_entries.is_empty() {
            let text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No executions yet",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Script execution logs will appear here",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let paragraph = ratatui::widgets::Paragraph::new(text)
                .alignment(Alignment::Center);

            f.render_widget(paragraph, area);
            return;
        }

        // Build list items from log entries
        let items: Vec<ListItem> = self
            .log_entries
            .iter()
            .rev() // Show newest first
            .map(|entry| {
                let (icon, color) = match entry.level {
                    LogLevel::Success => ("✓", Color::Green),
                    LogLevel::Error => ("✗", Color::Red),
                    LogLevel::Running => ("▶", Color::Yellow),
                    LogLevel::Info => ("•", Color::Blue),
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("[{}] ", entry.timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{} ", icon),
                        Style::default().fg(color).bold(),
                    ),
                    Span::styled(&entry.message, Style::default().fg(Color::White)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items);

        f.render_widget(list, area);
    }
}

impl Component for ExecutionLog {
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
        // Execution log doesn't handle events (not focusable yet)
        Ok(None)
    }

    fn update(&mut self, _state: &mut AppState, action: Action) -> Result<Option<Action>> {
        // Listen for execution events
        match action {
            Action::ScriptRunning(ref path) => {
                self.add_log(
                    format!("Executing {}", path),
                    LogLevel::Running,
                );
                Ok(Some(Action::Render))
            }
            Action::ScriptFinished(ref path, elapsed, _) => {
                self.add_log(
                    format!("Completed {} ({:.2}s)", path, elapsed as f64 / 1000.0),
                    LogLevel::Success,
                );
                Ok(Some(Action::Render))
            }
            Action::ScriptError(ref path, ref error, _) => {
                // Extract first line of error for log
                let error_summary = error.lines().next().unwrap_or("Unknown error");
                self.add_log(
                    format!("Failed {}: {}", path, error_summary),
                    LogLevel::Error,
                );
                Ok(Some(Action::Render))
            }
            Action::ClearOutput => {
                self.log_entries.clear();
                Ok(Some(Action::Render))
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, _state: &AppState) -> Result<()> {
        self.render_logs(f, area);
        Ok(())
    }
}

impl Default for ExecutionLog {
    fn default() -> Self {
        Self::new()
    }
}
