use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Clear, Paragraph, Wrap},
    layout::Size,
};
use tokio::sync::mpsc::UnboundedSender;
use std::path::PathBuf;

use super::Component;
use crate::{
    action::Action,
    app::{AppState, Script},
    infrastructure::Settings,
    tui::Frame,
};

/// File content cache entry
#[derive(Clone)]
struct FileCache {
    content_preview: Vec<String>,
    line_count: usize,
    file_size: u64,
}

/// Script preview panel that shows details and preview of the selected script
pub struct ScriptPreview {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    highlighted_script: Option<Script>,
    file_cache: Option<FileCache>,
    repository_base: PathBuf,
}

impl ScriptPreview {
    pub fn new(repository_base: PathBuf) -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            highlighted_script: None,
            file_cache: None,
            repository_base,
        }
    }

    /// Load file content asynchronously
    fn load_file_content(&mut self, script_path: &str) {
        let full_path = self.repository_base.join(script_path);
        let script_path_owned = script_path.to_string();
        let command_tx = self.command_tx.clone();

        tokio::spawn(async move {
            match tokio::fs::read_to_string(&full_path).await {
                Ok(content) => {
                    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                    let line_count = lines.len();
                    let content_preview: Vec<String> = lines.iter().take(20).cloned().collect();

                    // Get file size
                    let file_size = match tokio::fs::metadata(&full_path).await {
                        Ok(metadata) => metadata.len(),
                        Err(_) => 0,
                    };

                    let cache = FileCache {
                        content_preview,
                        line_count,
                        file_size,
                    };

                    if let Some(tx) = command_tx {
                        let _ = tx.send(Action::FileContentLoaded(script_path_owned, cache.content_preview.clone(), cache.line_count, cache.file_size));
                    }
                }
                Err(e) => {
                    log::error!("Failed to load file {}: {}", script_path_owned, e);
                }
            }
        });
    }

    fn render_placeholder(&self, f: &mut Frame<'_>, area: Rect) {
        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No script selected",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Select a script from the file tree",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "to see its details here.",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }

    /// Truncate a string to fit within the given width
    fn truncate_str(s: &str, max_width: usize) -> String {
        if s.len() > max_width && max_width > 1 {
            format!("{}…", &s[..max_width.saturating_sub(1)])
        } else {
            s.to_string()
        }
    }

    fn render_script_details(&self, f: &mut Frame<'_>, area: Rect, script: &Script) {
        // Conservative width calculation: subtract 4 for safety (borders, padding)
        let max_width = area.width.saturating_sub(4) as usize;

        let status_text = match script.state {
            crate::app::ScriptState::Finished => ("✓ Success", Color::Green),
            crate::app::ScriptState::Error => ("✗ Error", Color::Red),
            crate::app::ScriptState::Running => ("⟳ Running", Color::Yellow),
            crate::app::ScriptState::None => ("• Not Run", Color::Cyan),
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    Self::truncate_str(&script.relative_path, max_width.saturating_sub(6)),
                    Style::default().fg(Color::White)
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(status_text.0, Style::default().fg(status_text.1).bold()),
            ]),
        ];

        // Show file metadata if available
        if let Some(ref cache) = self.file_cache {
            lines.push(Line::from(vec![
                Span::styled("Size: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    Self::format_file_size(cache.file_size),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Lines: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    cache.line_count.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if let Some(elapsed) = script.elapsed {
            lines.push(Line::from(vec![
                Span::styled("Duration: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.2}s", elapsed as f64 / 1000.0),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if let Some(ref error) = script.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red).bold()),
            ]));
            lines.push(Line::from(""));

            // Show first few lines of error (truncated to fit)
            for (i, line) in error.lines().take(10).enumerate() {
                if i >= 10 {
                    lines.push(Line::from(Span::styled(
                        "... (truncated)",
                        Style::default().fg(Color::DarkGray).italic(),
                    )));
                    break;
                }
                lines.push(Line::from(Span::styled(
                    Self::truncate_str(line, max_width),
                    Style::default().fg(Color::Red),
                )));
            }
        } else if let Some(ref cache) = self.file_cache {
            // Show file preview
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "─".repeat(30),
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "File preview (first 20 lines)",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "─".repeat(30),
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));

            for (i, line) in cache.content_preview.iter().enumerate() {
                let line_num = format!("{:>4} │ ", i + 1);

                // Replace tabs with spaces (tabs expand to multiple columns when rendered)
                let line_with_spaces = line.replace('\t', "    ");

                // Truncate long lines to prevent overflow
                // Line number takes 7 chars (e.g. " 123 │ "), subtract a bit more for safety
                let content_width = max_width.saturating_sub(10);
                let truncated_line = Self::truncate_str(&line_with_spaces, content_width);

                lines.push(Line::from(vec![
                    Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                    Span::styled(truncated_line, Style::default().fg(Color::White)),
                ]));
            }

            if cache.line_count > 20 {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("... ({} more lines)", cache.line_count - 20),
                    Style::default().fg(Color::DarkGray).italic(),
                )));
            }
        } else {
            // Loading state
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Loading file preview...",
                Style::default().fg(Color::Yellow),
            )));
        }

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false })  // Disable wrap since we manually truncate
            .scroll((0, 0))
            .block(Block::default());  // Add block to ensure clipping

        f.render_widget(paragraph, area);
    }

    fn format_file_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;

        if bytes < KB {
            format!("{} bytes", bytes)
        } else if bytes < MB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        }
    }
}

impl Component for ScriptPreview {
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
        // Script preview doesn't handle events (not focusable yet)
        Ok(None)
    }

    fn update(&mut self, _state: &mut AppState, action: Action) -> Result<Option<Action>> {
        // Listen for highlighted script changes
        match action {
            Action::ScriptHighlighted(script) => {
                if let Some(ref new_script) = script {
                    // Check if this is a new script (different from current)
                    let should_load = self.highlighted_script.as_ref()
                        .map(|s| s.relative_path != new_script.relative_path)
                        .unwrap_or(true);

                    let script_path = new_script.relative_path.clone();

                    self.highlighted_script = script;

                    if should_load {
                        // Clear old cache
                        self.file_cache = None;
                        // Load new file content
                        self.load_file_content(&script_path);
                    }
                } else {
                    self.highlighted_script = None;
                    self.file_cache = None;
                }
                Ok(Some(Action::Render))
            }
            Action::FileContentLoaded(ref path, ref preview_lines, line_count, file_size) => {
                // Check if this content is for the currently highlighted script
                if let Some(ref script) = self.highlighted_script {
                    if script.relative_path == *path {
                        self.file_cache = Some(FileCache {
                            content_preview: preview_lines.clone(),
                            line_count,
                            file_size,
                        });
                        return Ok(Some(Action::Render));
                    }
                }
                Ok(None)
            }
            Action::ScriptRunning(ref path) |
            Action::ScriptFinished(ref path, ..) |
            Action::ScriptError(ref path, ..) => {
                // Update the highlighted script's state if it matches
                if let Some(ref mut script) = self.highlighted_script {
                    if script.relative_path == *path {
                        // The script state will be updated in the main app state
                        // Just trigger a re-render
                        return Ok(Some(Action::Render));
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, _state: &AppState) -> Result<()> {
        // Clear the area first to prevent ghosting/bleeding from previous renders
        f.render_widget(Clear, area);

        if let Some(ref script) = self.highlighted_script {
            self.render_script_details(f, area, script);
        } else {
            self.render_placeholder(f, area);
        }

        Ok(())
    }
}

impl Default for ScriptPreview {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}
