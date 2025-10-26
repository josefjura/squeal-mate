use std::path::PathBuf;

use color_eyre::eyre::{self, Result};

use crc::{Crc, CRC_32_ISO_HDLC};
use ratatui::{
    prelude::*,
    widgets::{block::Position, *},
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action, infrastructure::Settings, entries::EntryStatus,
    script_memory::ScriptDatabase, tui::Frame, utils::send_through_channel,
    services::{ActionDispatcher, MigrationService},
    infrastructure::FileExplorer,
};
use crate::{app::AppState, entries::ListEntry};
use std::sync::Arc;

pub struct List {
    base: PathBuf,
    current_directory: PathBuf,  // Navigation state - now owned by UI
    command_tx: Option<UnboundedSender<Action>>,
    dispatcher: Option<ActionDispatcher>,
    migration_service: Option<Arc<MigrationService>>,
    config: Settings,
    state: ListState,
    file_explorer: Arc<FileExplorer>,  // Simple file browsing (no domain abstractions)
    entries: Vec<ListEntry>,
    script_memory: ScriptDatabase,
    status_calculation_progress: Option<(usize, usize)>,  // (current, total) for CRC calculation
    pending_cursor_name: Option<String>,  // Name to position cursor on after loading
}

impl List {
    pub fn new(
        base: PathBuf,
        script_memory: ScriptDatabase,
    ) -> Result<Self> {
        let current_directory = base.clone();
        let file_explorer = Arc::new(FileExplorer::new(base.clone())?);

        Ok(Self {
            state: ListState::default().with_selected(Some(0)),
            command_tx: None,
            dispatcher: None,
            migration_service: None,
            config: Settings::default(),
            entries: Vec::new(),  // Will be populated via refresh_entries()
            script_memory,
            file_explorer,
            base: base.clone(),
            current_directory,
            status_calculation_progress: None,
            pending_cursor_name: None,
        })
    }

    pub fn set_migration_service(&mut self, service: Arc<MigrationService>) {
        self.migration_service = Some(service);
    }

    /// Refresh the entries list from the current directory
    /// This now spawns an async task and returns immediately
    pub fn refresh_entries(&mut self) -> eyre::Result<()> {
        // Show loading state immediately
        self.entries = vec![ListEntry {
            name: "Loading...".to_string(),
            relative_path: "".to_string(),
            selected: false,
            is_directory: false,
            status: EntryStatus::Loading,
        }];

        // Dispatch loading action
        if let Some(ref dispatcher) = self.dispatcher {
            dispatcher.dispatch(Action::EntriesLoading);
        }

        // Spawn async task to load entries
        let current_dir = self.current_directory.clone();
        let root_dir = self.base.clone();
        let file_explorer = self.file_explorer.clone();
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            // Use FileExplorer to get all entries (files + directories)
            let result = file_explorer.list_directory(&current_dir).await;

            let entries = match result {
                Ok(explorer_entries) => {
                    // Convert from FileExplorer entries to ListEntry
                    explorer_entries
                        .into_iter()
                        .map(|entry| {
                            let relative_path = entry.path
                                .strip_prefix(&root_dir)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| entry.name.clone());

                            ListEntry {
                                name: entry.name,
                                relative_path,
                                selected: false,
                                is_directory: entry.is_directory,
                                status: EntryStatus::Unknown,
                            }
                        })
                        .collect()
                }
                Err(e) => {
                    log::error!("Failed to list directory: {}", e);
                    Vec::new()
                }
            };

            // Send results back via action
            if let Some(dispatcher) = dispatcher {
                dispatcher.dispatch(Action::EntriesLoaded(entries));
            }
        });

        Ok(())
    }

    pub fn cursor_up(&mut self) {
        if let Some(position) = self.state.selected() {
            if position > 0 {
                self.state.select(Some(position - 1))
            }
        }
    }

    pub fn cursor_down(&mut self, entries_len: usize) {
        if let Some(position) = self.state.selected() {
            if position < entries_len - 1 {
                self.state.select(Some(position + 1))
            }
        }
    }

    pub fn go_to_top(&mut self) {
        self.state.select(Some(0));
    }

    pub fn go_to_bottom(&mut self, entries_len: usize) {
        self.state.select(Some(entries_len - 1));
    }

    pub fn get_selection(&self) -> Option<&ListEntry> {
        if let Some(selected) = self.state.selected() {
            self.entries.get(selected)
        } else {
            None
        }
    }

    pub fn open_selected_directory(&mut self) -> eyre::Result<()> {
        let entry = self.get_selection().cloned();

        if let Some(ListEntry {
            is_directory: true,
            name,
            ..
        }) = entry
        {
            // Navigate by updating current_directory state
            self.current_directory = self.current_directory.join(&name);
            self.refresh_entries()?;
            // Note: CalculateEntryStatus will be dispatched when EntriesLoaded action is received
        }

        Ok(())
    }

    pub fn leave_current_directory(&mut self) -> eyre::Result<()> {
        // Navigate up by updating current_directory state
        if let Some(parent) = self.current_directory.parent() {
            // Save the old directory name to position cursor on it after loading
            self.pending_cursor_name = self.current_directory
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());

            self.current_directory = parent.to_path_buf();
            self.refresh_entries()?;
            // Note: Cursor will be positioned when EntriesLoaded action is received
        }

        Ok(())
    }

    pub fn select_current(&mut self, state: &mut AppState) {
        let entry = self.get_selection().cloned();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        if entry.is_directory {
            // Get children via repository trait
            let repo_base = self.base.clone();
            let rel_path = repo_base.join(&entry.relative_path);

            // NOTE: Using block_on here is acceptable because:
            // 1. This operation is fast (just reading directory listing from memory)
            // 2. It's triggered by explicit user action (selection)
            // 3. Alternative would add significant complexity for minimal UX benefit
            let handle = tokio::runtime::Handle::current();
            let root_dir = self.base.clone();
            let file_explorer = self.file_explorer.clone();
            let items = handle.block_on(async {
                match file_explorer.list_sql_files(&rel_path).await {
                    Ok(paths) => paths.into_iter()
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>(),
                    Err(e) => {
                        log::error!("Failed to get children for directory {}: {}", entry.relative_path, e);
                        vec![]
                    }
                }
            });

            state.toggle_many(&items);
        } else {
            state.toggle(entry.relative_path);
        }
    }

    pub fn unselect_current(&mut self, state: &mut AppState) {
        let entry = self.get_selection().cloned();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        if entry.is_directory {
            // Use async task to get children via trait method
            let repo_base = self.base.clone();
            let rel_path = repo_base.join(&entry.relative_path);

            // NOTE: Using block_on here is acceptable because:
            // 1. This operation is fast (just reading directory listing from memory)
            // 2. It's triggered by explicit user action (selection)
            // 3. Alternative would add significant complexity for minimal UX benefit
            let handle = tokio::runtime::Handle::current();
            let root_dir = self.base.clone();
            let file_explorer = self.file_explorer.clone();
            let items = handle.block_on(async {
                match file_explorer.list_sql_files(&rel_path).await {
                    Ok(paths) => paths.into_iter()
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>(),
                    Err(e) => {
                        log::error!("Failed to get children for directory {}: {}", entry.relative_path, e);
                        vec![]
                    }
                }
            });

            state.remove_many(&items);
        } else {
            state.remove(entry.relative_path);
        }
    }

    pub fn unselect_all(&mut self, state: &mut AppState) {
        state.selected.clear()
    }

    pub fn select_all_after(&mut self, state: &mut AppState) {
        let entry = self.get_selection().cloned();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        // NOTE: Using block_on here is acceptable because:
        // 1. This operation is fast (just reading directory listing from memory)
        // 2. It's triggered by explicit user action (selection)
        // 3. Alternative would add significant complexity for minimal UX benefit
        let handle = tokio::runtime::Handle::current();
        let root_dir = self.base.clone();
        let current_dir = self.current_directory.clone();
        let file_explorer = self.file_explorer.clone();
        let after_name = entry.name.clone();

        let entries = handle.block_on(async {
            match file_explorer.list_sql_files(&current_dir).await {
                Ok(paths) => {
                    // Filter to only files after the selected one (alphabetically)
                    paths.into_iter()
                        .filter(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|name| name > after_name.as_str())
                                .unwrap_or(false)
                        })
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                }
                Err(e) => {
                    log::error!("Failed to get scripts after {}: {}", after_name, e);
                    vec![]
                }
            }
        });

        state.add_many(&entries);
    }

    pub fn select_all_after_in_directory(&mut self, state: &mut AppState) {
        let entry = self.get_selection().cloned();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        // NOTE: Using block_on here is acceptable because:
        // 1. This operation is fast (just reading directory listing from memory)
        // 2. It's triggered by explicit user action (selection)
        // 3. Alternative would add significant complexity for minimal UX benefit
        let handle = tokio::runtime::Handle::current();
        let root_dir = self.base.clone();
        let current_dir = self.current_directory.clone();
        let file_explorer = self.file_explorer.clone();
        let after_name = entry.name.clone();

        let entries = handle.block_on(async {
            match file_explorer.list_sql_files(&current_dir).await {
                Ok(paths) => {
                    // Filter to only files after the selected one (alphabetically)
                    paths.into_iter()
                        .filter(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|name| name > after_name.as_str())
                                .unwrap_or(false)
                        })
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                }
                Err(e) => {
                    log::error!("Failed to get scripts after {} in directory: {}", after_name, e);
                    vec![]
                }
            }
        });

        state.add_many(&entries);
    }

    pub fn select_all_in_directory(&mut self, state: &mut AppState) {
        // NOTE: Using block_on here is acceptable because:
        // 1. This operation is fast (just reading directory listing from memory)
        // 2. It's triggered by explicit user action (selection)
        // 3. Alternative would add significant complexity for minimal UX benefit
        let handle = tokio::runtime::Handle::current();
        let root_dir = self.base.clone();
        let current_dir = self.current_directory.clone();
        let file_explorer = self.file_explorer.clone();

        let entries = handle.block_on(async {
            match file_explorer.list_sql_files(&current_dir).await {
                Ok(paths) => {
                    paths.into_iter()
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                }
                Err(e) => {
                    log::error!("Failed to get scripts in directory: {}", e);
                    vec![]
                }
            }
        });

        state.add_many(&entries);
    }
}

impl Component for List {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        let dispatcher = ActionDispatcher::new(tx.clone());
        self.dispatcher = Some(dispatcher);
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Settings) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn init(&mut self, _area: ratatui::prelude::Size) -> Result<()> {
        // Load entries now that dispatcher is set up
        self.refresh_entries()?;
        Ok(())
    }

    fn update(&mut self, state: &mut AppState, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::CursorUp => {
                self.cursor_up();
                return Ok(None);
            }
            Action::CursorDown => {
                self.cursor_down(self.entries.len());
                return Ok(None);
            }
            Action::CursorToTop => {
                self.go_to_top();
                return Ok(None);
            }
            Action::CursorToBottom => {
                self.go_to_bottom(self.entries.len());
                return Ok(None);
            }
            Action::DirectoryOpenSelected => {
                self.open_selected_directory()?;
                return Ok(None);
            }
            Action::DirectoryLeave => {
                self.leave_current_directory()?;
                return Ok(None);
            }
            Action::SelectCurrent => {
                self.select_current(state);
                return Ok(None);
            }
            Action::UnselectCurrent => {
                self.unselect_current(state);
                return Ok(None);
            }
            Action::UnselectAll => {
                self.unselect_all(state);
                return Ok(None);
            }
            Action::SelectAllAfter => {
                self.select_all_after(state);
                return Ok(None);
            }
            Action::SelectAllAfterInDirectory => {
                self.select_all_after_in_directory(state);
            }
            Action::SelectAllInDirectory => {
                self.select_all_in_directory(state);
                return Ok(None);
            }
            Action::CalculateEntryStatus => {
                // Reset progress tracking
                let file_count = self.entries.iter().filter(|e| !e.is_directory).count();

                // Only show progress if there are files to process
                if file_count > 0 {
                    self.status_calculation_progress = Some((0, file_count));
                } else {
                    self.status_calculation_progress = None;
                }

                // Use MigrationService if available, otherwise fall back to legacy
                if let (Some(migration_service), Some(dispatcher)) =
                    (&self.migration_service, &self.dispatcher) {

                    use crate::domain::ScriptPath;

                    // Convert entries to ScriptPaths (only files, not directories)
                    let script_paths: Vec<ScriptPath> = self.entries
                        .iter()
                        .filter(|e| !e.is_directory)
                        .filter_map(|e| {
                            ScriptPath::new(e.relative_path.clone()).ok()
                        })
                        .collect();

                    // Dispatch directory statuses immediately
                    for entry in &self.entries {
                        if entry.is_directory {
                            dispatcher.dispatch(Action::EntryStatusChanged(
                                entry.relative_path.clone(),
                                EntryStatus::Directory,
                            ));
                        }
                    }

                    // Use service to calculate file statuses asynchronously
                    migration_service.calculate_statuses(script_paths, dispatcher);
                } else {
                    // Legacy fallback (old implementation)
                    log::warn!("MigrationService not available, using legacy status calculation");
                    let channel: Option<UnboundedSender<Action>> = self.command_tx.clone();
                    let memory = self.script_memory.clone();
                    let base = self.base.clone();
                    let entries: Vec<_> = self.entries.clone();
                    tokio::spawn(async move {
                        for entry in entries {
                            if entry.is_directory {
                                send_through_channel(
                                    &channel,
                                    Action::EntryStatusChanged(
                                        entry.relative_path,
                                        EntryStatus::Directory,
                                    ),
                                );
                                continue;
                            }
                            let full_path = base.join(&entry.relative_path);

                            let content = tokio::fs::read_to_string(full_path).await;
                            match content {
                                core::result::Result::Ok(content) => {
                                    let hasher = Crc::<u32>::new(&CRC_32_ISO_HDLC);
                                    let crc = hasher.checksum(content.as_bytes());
                                    let status = memory.get_file_status(&entry.relative_path, &crc);

                                    if let core::result::Result::Ok(status) = status {
                                        send_through_channel(
                                            &channel,
                                            Action::EntryStatusChanged(entry.relative_path, status),
                                        )
                                    }
                                }
                                Err(e) => {
                                    log::error!("Error reading file {} : {}", e, entry.relative_path);
                                }
                            }
                        }
                    });
                }

                return Ok(None);
            }
            Action::EntryStatusChanged(path, status) => {
                let index = self
                    .entries
                    .iter()
                    .position(|e| e.relative_path == path)
                    .unwrap();

                self.entries[index].status = status.clone();

                return Ok(None);
            }
            Action::EntriesLoaded(entries) => {
                // Update entries from async task
                self.entries = entries;

                // Position cursor based on pending_cursor_name if set
                if let Some(name) = self.pending_cursor_name.take() {
                    // Find the entry with the matching name
                    let index = self.entries.iter().position(|e| e.name == name);
                    if let Some(idx) = index {
                        self.state.select(Some(idx));
                    } else if !self.entries.is_empty() {
                        self.state.select(Some(0));
                    } else {
                        self.state.select(None);
                    }
                } else {
                    // No pending cursor, use default logic
                    if !self.entries.is_empty() {
                        if self.state.selected().is_none() {
                            self.state.select(Some(0));
                        }
                    } else {
                        self.state.select(None);
                    }
                }

                // Now that entries are loaded, calculate their statuses
                if let Some(ref dispatcher) = self.dispatcher {
                    dispatcher.dispatch(Action::CalculateEntryStatus);
                }

                return Ok(None);
            }
            Action::StatusCalculationProgress(current, total) => {
                // Update progress tracking
                self.status_calculation_progress = Some((current, total));

                // Clear progress when complete
                if current >= total {
                    self.status_calculation_progress = None;
                }

                // Request a render to show the progress update
                return Ok(Some(Action::Render));
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &AppState) -> Result<()> {
        // Add constraint for progress bar if calculating statuses
        let constraints = if self.status_calculation_progress.is_some() {
            vec![Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]
        } else {
            vec![Constraint::Length(1), Constraint::Fill(1)]
        };

        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let path_span = Span::raw(
            self.current_directory
                .display()
                .to_string(),
        );
        let path_draw = Line::default().spans(vec![path_span]);

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let name = entry.name.clone();
                let decoratation = match entry.status {
                    EntryStatus::Finished(true) => ("\u{02705}", Style::new().fg(Color::Green)),
                    EntryStatus::Finished(false) => ("\u{0274E}", Style::new().fg(Color::Red)),
                    EntryStatus::Changed => ("\u{02755}", Style::new().fg(Color::Rgb(255, 165, 0))),
                    EntryStatus::Unknown => ("\u{02754}", Style::default()),
                    EntryStatus::NeverStarted => {
                        ("\u{1F195}", Style::new().fg(Color::Rgb(255, 165, 0)))
                    }
                    EntryStatus::Directory => ("", Style::default().bg(Color::LightBlue)),
                    EntryStatus::Loading => ("\u{231B}", Style::new().fg(Color::Yellow)),  // ⌛ hourglass
                };
                let selected = state
                    .selected
                    .iter()
                    .any(|s| s.relative_path == entry.relative_path);

                let style = match (selected, entry.is_directory) {
                    (_, true) => Style::new().light_blue(),
                    (true, false) => Style::new().green(),
                    (false, false) => Style::new().white(),
                };

                let symbol = Span::styled(decoratation.0, decoratation.1);

                let line = Line::default().spans(vec![
                    symbol,
                    Span::styled(" ", style),
                    Span::styled(name, style),
                ]);

                let list_item = ListItem::new(line).style(style);
                list_item
            })
            .collect();

        let list_draw = ratatui::widgets::List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title_position(Position::Bottom)
                    .title_alignment(Alignment::Right)
                    .title("Press h for help"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true);

        f.render_widget(path_draw, rects[0]);
        f.render_stateful_widget(list_draw, rects[1], &mut self.state);

        // Render progress indicator if status calculation is in progress
        if let Some((current, total)) = self.status_calculation_progress {
            let percentage = if total > 0 {
                (current as f64 / total as f64 * 100.0) as u16
            } else {
                0
            };

            let progress_text = format!(" Calculating checksums: {}/{} ({}%) ", current, total, percentage);
            let progress_line = Line::from(vec![
                Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                Span::styled(progress_text, Style::default().fg(Color::Cyan)),
            ]);

            f.render_widget(progress_line, rects[2]);
        }

        Ok(())
    }
}
