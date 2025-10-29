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
    ui::tree_state::TreeState,
};
use crate::{app::AppState, entries::ListEntry};
use std::sync::Arc;

pub struct List {
    base: PathBuf,
    command_tx: Option<UnboundedSender<Action>>,
    dispatcher: Option<ActionDispatcher>,
    migration_service: Option<Arc<MigrationService>>,
    config: Settings,
    widget_state: ListState,  // Ratatui widget state (for scrolling)
    tree_state: TreeState,  // Tree view state with hierarchy
    file_explorer: Arc<FileExplorer>,  // Simple file browsing (no domain abstractions)
    script_memory: ScriptDatabase,
}

impl List {
    pub fn new(
        base: PathBuf,
        script_memory: ScriptDatabase,
    ) -> Result<Self> {
        let file_explorer = Arc::new(FileExplorer::new(base.clone())?);
        let tree_state = TreeState::new(base.clone());

        Ok(Self {
            widget_state: ListState::default().with_selected(Some(0)),
            command_tx: None,
            dispatcher: None,
            migration_service: None,
            config: Settings::default(),
            script_memory,
            file_explorer,
            base: base.clone(),
            tree_state,
        })
    }

    pub fn set_migration_service(&mut self, service: Arc<MigrationService>) {
        self.migration_service = Some(service);
    }

    /// Refresh the entire tree recursively
    /// This loads all files and directories from the root
    pub fn refresh_entries(&mut self) -> eyre::Result<()> {
        // Dispatch loading action
        if let Some(ref dispatcher) = self.dispatcher {
            dispatcher.dispatch(Action::EntriesLoading);
        }

        // Spawn async task to load ALL entries recursively
        let root_dir = self.base.clone();
        let file_explorer = self.file_explorer.clone();
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            // Recursively get all entries from root
            let result = Self::load_tree_recursive(&file_explorer, &root_dir, &root_dir).await;

            let entries = match result {
                Ok(entries) => entries,
                Err(e) => {
                    log::error!("Failed to load tree: {}", e);
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

    /// Recursively load all entries for the tree
    fn load_tree_recursive<'a>(
        explorer: &'a FileExplorer,
        current_dir: &'a PathBuf,
        root_dir: &'a PathBuf,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = eyre::Result<Vec<ListEntry>>> + Send + 'a>> {
        Box::pin(async move {
        let mut all_entries = Vec::new();

        // Get entries in current directory
        let explorer_entries = explorer.list_directory(current_dir).await?;

        for entry in explorer_entries {
            let relative_path = entry.path
                .strip_prefix(root_dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| entry.name.clone());

            all_entries.push(ListEntry {
                name: entry.name.clone(),
                relative_path: relative_path.clone(),
                selected: false,
                is_directory: entry.is_directory,
                status: EntryStatus::Unknown,
            });

            // Recursively load subdirectories
            if entry.is_directory {
                match Self::load_tree_recursive(explorer, &entry.path, root_dir).await {
                    Ok(children) => all_entries.extend(children),
                    Err(e) => log::warn!("Failed to load subdirectory {}: {}", entry.name, e),
                }
            }
        }

        Ok(all_entries)
        })
    }

    pub fn cursor_up(&mut self) {
        self.tree_state.cursor_up();
        // Sync widget state with tree state
        self.widget_state.select(Some(self.tree_state.cursor()));
    }

    pub fn cursor_down(&mut self, entries_len: usize) {
        self.tree_state.cursor_down(entries_len);
        // Sync widget state with tree state
        self.widget_state.select(Some(self.tree_state.cursor()));
    }

    pub fn go_to_top(&mut self) {
        self.tree_state.cursor_to_top();
        self.widget_state.select(Some(self.tree_state.cursor()));
    }

    pub fn go_to_bottom(&mut self, entries_len: usize) {
        self.tree_state.cursor_to_bottom(entries_len);
        self.widget_state.select(Some(self.tree_state.cursor()));
    }

    pub fn get_selection(&mut self) -> Option<ListEntry> {
        self.tree_state.selected_node().map(|n| n.entry.clone())
    }

    /// Expand or collapse the selected folder (tree view)
    pub fn open_selected_directory(&mut self) -> eyre::Result<()> {
        // In tree view, "opening" a directory means expanding it
        if self.tree_state.toggle_current_expansion() {
            // Update widget state to reflect new visible rows
            self.widget_state.select(Some(self.tree_state.cursor()));
        }
        Ok(())
    }

    /// Collapse current folder or move to parent (tree view)
    pub fn leave_current_directory(&mut self) -> eyre::Result<()> {
        // In tree view, we can collapse the current folder if it's expanded
        // For now, just toggle (same as open)
        if self.tree_state.toggle_current_expansion() {
            self.widget_state.select(Some(self.tree_state.cursor()));
        }
        Ok(())
    }

    pub fn select_current(&mut self, state: &mut AppState) {
        let entry = self.get_selection();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        if entry.is_directory {
            // Spawn async task to get directory children
            let repo_base = self.base.clone();
            let rel_path = repo_base.join(&entry.relative_path);
            let root_dir = self.base.clone();
            let file_explorer = self.file_explorer.clone();
            let dispatcher = self.dispatcher.clone();
            let entry_path = entry.relative_path.clone();

            tokio::spawn(async move {
                match file_explorer.list_sql_files(&rel_path).await {
                    Ok(paths) => {
                        let items: Vec<String> = paths.into_iter()
                            .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();

                        if let Some(dispatcher) = dispatcher {
                            dispatcher.dispatch(Action::ToggleSelection(items));
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get children for directory {}: {}", entry_path, e);
                    }
                }
            });
        } else {
            state.toggle(entry.relative_path);
        }
    }

    pub fn unselect_current(&mut self, state: &mut AppState) {
        let entry = self.get_selection();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        if entry.is_directory {
            // Spawn async task to get directory children
            let repo_base = self.base.clone();
            let rel_path = repo_base.join(&entry.relative_path);
            let root_dir = self.base.clone();
            let file_explorer = self.file_explorer.clone();
            let dispatcher = self.dispatcher.clone();
            let entry_path = entry.relative_path.clone();

            tokio::spawn(async move {
                match file_explorer.list_sql_files(&rel_path).await {
                    Ok(paths) => {
                        let items: Vec<String> = paths.into_iter()
                            .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();

                        if let Some(dispatcher) = dispatcher {
                            dispatcher.dispatch(Action::RemoveSelection(items));
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get children for directory {}: {}", entry_path, e);
                    }
                }
            });
        } else {
            state.remove(entry.relative_path);
        }
    }

    pub fn unselect_all(&mut self, state: &mut AppState) {
        state.selected.clear()
    }

    pub fn select_all_after(&mut self, _state: &mut AppState) {
        let entry = self.get_selection();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        // In tree view, select all files after current one (recursively)
        let root_dir = self.base.clone();
        let file_explorer = self.file_explorer.clone();
        let after_name = entry.name.clone();
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            match file_explorer.list_sql_files(&root_dir).await {
                Ok(paths) => {
                    // Filter to only files after the selected one (alphabetically)
                    let entries: Vec<String> = paths.into_iter()
                        .filter(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|name| name > after_name.as_str())
                                .unwrap_or(false)
                        })
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();

                    if let Some(dispatcher) = dispatcher {
                        dispatcher.dispatch(Action::AddSelection(entries));
                    }
                }
                Err(e) => {
                    log::error!("Failed to get scripts after {}: {}", after_name, e);
                }
            }
        });
    }

    pub fn select_all_after_in_directory(&mut self, _state: &mut AppState) {
        let entry = self.get_selection();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        // In tree view, same as select_all_after (no directory context)
        let root_dir = self.base.clone();
        let file_explorer = self.file_explorer.clone();
        let after_name = entry.name.clone();
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            match file_explorer.list_sql_files(&root_dir).await {
                Ok(paths) => {
                    // Filter to only files after the selected one (alphabetically)
                    let entries: Vec<String> = paths.into_iter()
                        .filter(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|name| name > after_name.as_str())
                                .unwrap_or(false)
                        })
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();

                    if let Some(dispatcher) = dispatcher {
                        dispatcher.dispatch(Action::AddSelection(entries));
                    }
                }
                Err(e) => {
                    log::error!("Failed to get scripts after {} in directory: {}", after_name, e);
                }
            }
        });
    }

    pub fn select_all_in_directory(&mut self, _state: &mut AppState) {
        // In tree view, select all files in entire tree
        let root_dir = self.base.clone();
        let file_explorer = self.file_explorer.clone();
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            match file_explorer.list_sql_files(&root_dir).await {
                Ok(paths) => {
                    let entries: Vec<String> = paths.into_iter()
                        .filter_map(|p| p.strip_prefix(&root_dir).ok().map(|p| p.to_path_buf()))
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();

                    if let Some(dispatcher) = dispatcher {
                        dispatcher.dispatch(Action::AddSelection(entries));
                    }
                }
                Err(e) => {
                    log::error!("Failed to get scripts in directory: {}", e);
                }
            }
        });
    }

    /// Get the currently highlighted script for the preview panel
    fn get_highlighted_script(&mut self, state: &AppState) -> Option<Action> {
        use crate::app::{Script, ScriptState};

        // Get the currently selected entry
        let entry = self.get_selection()?;

        // Only highlight files, not directories
        if entry.is_directory {
            return Some(Action::ScriptHighlighted(None));
        }

        // Check if script is in the selected/executed list
        let script = if let Some(existing) = state.selected
            .iter()
            .find(|s| s.relative_path == entry.relative_path)
        {
            // Use existing script with execution state
            existing.clone()
        } else {
            // Create new script entry for preview
            Script {
                relative_path: entry.relative_path.clone(),
                state: ScriptState::None,
                error: None,
                elapsed: None,
            }
        };

        Some(Action::ScriptHighlighted(Some(script)))
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
                return Ok(self.get_highlighted_script(state));
            }
            Action::CursorDown => {
                let len = self.tree_state.flattened().len();
                self.cursor_down(len);
                return Ok(self.get_highlighted_script(state));
            }
            Action::CursorToTop => {
                self.go_to_top();
                return Ok(self.get_highlighted_script(state));
            }
            Action::CursorToBottom => {
                let len = self.tree_state.flattened().len();
                self.go_to_bottom(len);
                return Ok(self.get_highlighted_script(state));
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
                // Get all entries from tree
                let entries = self.tree_state.entries();
                let _file_count = entries.iter().filter(|e| !e.is_directory).count();

                // TODO: Add crc_progress field to List struct for progress display
                // For now, we'll just skip progress tracking

                // Use MigrationService if available, otherwise fall back to legacy
                if let (Some(migration_service), Some(dispatcher)) =
                    (&self.migration_service, &self.dispatcher) {

                    use crate::domain::ScriptPath;

                    // Convert entries to ScriptPaths (only files, not directories)
                    let script_paths: Vec<ScriptPath> = entries
                        .iter()
                        .filter(|e| !e.is_directory)
                        .filter_map(|e| {
                            ScriptPath::new(e.relative_path.clone()).ok()
                        })
                        .collect();

                    // Dispatch directory statuses immediately
                    for entry in &entries {
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
                    let entries: Vec<_> = entries.iter().map(|e| (*e).clone()).collect();
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
                self.tree_state.update_entry_status(&path, status);
                return Ok(None);
            }
            Action::EntriesLoaded(entries) => {
                // Build tree from flat entries list
                self.tree_state.build_from_entries(entries);

                // Sync widget state cursor with tree state cursor
                self.widget_state.select(Some(self.tree_state.cursor()));

                // Now that entries are loaded, calculate their statuses
                if let Some(ref dispatcher) = self.dispatcher {
                    dispatcher.dispatch(Action::CalculateEntryStatus);
                }

                // Highlight the first entry for the preview panel
                return Ok(self.get_highlighted_script(state));
            }
            Action::StatusCalculationProgress(current, total) => {
                // Store progress for rendering (will add field to List struct)
                // For now, just trigger render
                // TODO: Add crc_progress field to List
                let _ = (current, total); // Suppress unused warning

                // Request a render to show the progress update
                return Ok(Some(Action::Render));
            }
            Action::AddSelection(paths) => {
                state.add_many(&paths);
                return Ok(None);
            }
            Action::RemoveSelection(paths) => {
                state.remove_many(&paths);
                return Ok(None);
            }
            Action::ToggleSelection(paths) => {
                state.toggle_many(&paths);
                return Ok(None);
            }
            Action::ScriptRun(skip_errors) => {
                use crate::app::ScriptState;

                // Find first script that hasn't been run yet
                let first_not_run_entry = state
                    .selected
                    .iter()
                    .find(|f| f.state == ScriptState::None)
                    .cloned();

                if first_not_run_entry.is_none() {
                    return Ok(None);
                }
                let entry = first_not_run_entry.unwrap();

                // Get the migration service and dispatcher
                let migration_service = match &self.migration_service {
                    Some(svc) => svc.clone(),
                    None => {
                        log::error!("MigrationService not available in List component");
                        return Ok(None);
                    }
                };

                let dispatcher = match &self.dispatcher {
                    Some(d) => d.clone(),
                    None => {
                        log::error!("ActionDispatcher not available in List component");
                        return Ok(None);
                    }
                };

                let full_path = self.base.join(&entry.relative_path);
                let script_path = entry.relative_path.clone();

                // Spawn async execution using service layer
                tokio::spawn(async move {
                    use crate::domain::{MigrationScript, ScriptPath};

                    // Read the script file
                    let content = match tokio::fs::read_to_string(&full_path).await {
                        Ok(c) => c,
                        Err(err) => {
                            dispatcher.dispatch(Action::ScriptError(
                                script_path,
                                err.to_string(),
                                None,
                            ));
                            return;
                        }
                    };

                    // Create domain objects
                    let path = match ScriptPath::new(script_path.clone()) {
                        Ok(p) => p,
                        Err(e) => {
                            dispatcher.dispatch(Action::ScriptError(
                                script_path,
                                format!("Invalid script path: {}", e),
                                None,
                            ));
                            return;
                        }
                    };

                    let script = MigrationScript::new(path, content);

                    // Use the service to execute
                    match migration_service.execute_script(&script, &dispatcher).await {
                        Ok(_) => {
                            // Service handles notifications, just trigger next script
                            dispatcher.dispatch(Action::ScriptRun(skip_errors));
                        }
                        Err(e) => {
                            log::error!("Script execution failed: {}", e);
                            // Error notifications already sent by service
                            if skip_errors {
                                dispatcher.dispatch(Action::ScriptRun(skip_errors));
                            }
                        }
                    }
                });

                return Ok(None);
            }
            Action::ScriptRunning(ref path) => {
                use crate::app::ScriptState;

                // Update script state to Running
                state
                    .selected
                    .iter_mut()
                    .filter(|s| s.relative_path == *path)
                    .for_each(|s| s.state = ScriptState::Running);

                return Ok(Some(Action::Render));
            }
            Action::ScriptFinished(ref path, elapsed, _) => {
                use crate::app::ScriptState;

                // Update script state to Finished
                state
                    .selected
                    .iter_mut()
                    .filter(|s| s.relative_path == *path)
                    .for_each(|s| {
                        s.state = ScriptState::Finished;
                        s.elapsed = Some(elapsed);
                    });

                return Ok(Some(Action::Render));
            }
            Action::ScriptError(ref path, ref error, _) => {
                use crate::app::ScriptState;

                // Update script state to Error
                state
                    .selected
                    .iter_mut()
                    .filter(|s| s.relative_path == *path)
                    .for_each(|s| {
                        s.state = ScriptState::Error;
                        s.error = Some(error.clone());
                    });

                return Ok(Some(Action::Render));
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &AppState) -> Result<()> {
        // Simpler layout for tree view (no progress bar for now)
        let constraints = vec![Constraint::Length(1), Constraint::Fill(1)];

        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Show root path
        let path_span = Span::raw(format!("📂 {}", self.base.display()));
        let path_draw = Line::default().spans(vec![path_span]);

        // Get flattened tree view
        let flattened = self.tree_state.flattened();

        let items: Vec<ListItem> = flattened
            .iter()
            .map(|node| {
                let entry = &node.entry;
                let name = entry.name.clone();
                let decoratation = match entry.status {
                    // ✓ Script ran successfully
                    EntryStatus::Finished(true) => ("\u{2713}", Style::new().fg(Color::Green)),
                    // ✗ Script failed
                    EntryStatus::Finished(false) => ("\u{2717}", Style::new().fg(Color::Red)),
                    // ⚠ Script was modified since last run (warning)
                    EntryStatus::Changed => ("\u{26A0}", Style::new().fg(Color::Yellow)),
                    // ? Unknown status (not yet checked)
                    EntryStatus::Unknown => ("?", Style::new().fg(Color::Gray)),
                    // • Never run before (bullet point - neutral)
                    EntryStatus::NeverStarted => {
                        ("\u{2022}", Style::new().fg(Color::Cyan))
                    }
                    // (directory has blue background, no icon needed)
                    EntryStatus::Directory => (" ", Style::default().bg(Color::LightBlue)),
                    // ⧗ Loading/calculating status
                    EntryStatus::Loading => ("\u{29D7}", Style::new().fg(Color::Yellow)),
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

                // Build indentation and tree structure
                let indent = "  ".repeat(node.depth);

                // Expand/collapse icon for directories
                let tree_icon = if entry.is_directory {
                    if node.expanded {
                        if node.has_children {
                            "▼ " // Expanded folder with children
                        } else {
                            "▶ " // Expanded empty folder
                        }
                    } else {
                        if node.has_children {
                            "▶ " // Collapsed folder with children
                        } else {
                            "▷ " // Empty folder
                        }
                    }
                } else {
                    "  " // File (no icon, just spacing)
                };

                // Show selection count for directories
                let folder_badge = if let Some(count) = node.selected_count {
                    if count > 0 {
                        format!(" [{}]", count)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                let symbol = Span::styled(decoratation.0, decoratation.1);

                let line = Line::default().spans(vec![
                    Span::raw(indent),
                    Span::styled(tree_icon, Style::default().fg(Color::DarkGray)),
                    symbol,
                    Span::styled(" ", style),
                    Span::styled(name, style),
                    Span::styled(folder_badge, Style::default().fg(Color::Yellow)),
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
        f.render_stateful_widget(list_draw, rects[1], &mut self.widget_state);

        Ok(())
    }
}
