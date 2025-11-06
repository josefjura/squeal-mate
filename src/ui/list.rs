use std::path::PathBuf;

use color_eyre::eyre::{self, Result};

use ratatui::{
    prelude::*,
    widgets::{block::Position, *},
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    entries::EntryStatus,
    infrastructure::{FileExplorer, FileSystem},
    infrastructure::Settings,
    script_memory::ScriptDatabase,
    services::{ActionDispatcher, MigrationService},
    tui::Frame,
    ui::tree_state::TreeState,
    utils::send_through_channel,
};
use crate::{app::AppState, entries::ListEntry};
use std::sync::Arc;

pub struct List {
    base: PathBuf,
    command_tx: Option<UnboundedSender<Action>>,
    dispatcher: Option<ActionDispatcher>,
    migration_service: Option<Arc<MigrationService>>,
    config: Settings,
    widget_state: ListState,          // Ratatui widget state (for scrolling)
    tree_state: TreeState,            // Tree view state with hierarchy
    filesystem: Arc<dyn FileSystem>,  // Abstraction for file browsing (allows mocking)
    script_memory: ScriptDatabase,
    is_searching: bool, // Flag for showing "Searching..." indicator
}

impl List {
    /// Create a new List with the real filesystem
    pub fn new(base: PathBuf, script_memory: ScriptDatabase) -> Result<Self> {
        let filesystem = Arc::new(FileExplorer::new(base.clone())?) as Arc<dyn FileSystem>;
        Self::new_with_filesystem(base, script_memory, filesystem)
    }

    /// Create a new List with a custom filesystem (for testing)
    pub fn new_with_filesystem(
        base: PathBuf,
        script_memory: ScriptDatabase,
        filesystem: Arc<dyn FileSystem>,
    ) -> Result<Self> {
        let tree_state = TreeState::new(base.clone());

        Ok(Self {
            widget_state: ListState::default().with_selected(Some(0)),
            command_tx: None,
            dispatcher: None,
            migration_service: None,
            config: Settings::default(),
            script_memory,
            filesystem,
            base: base.clone(),
            tree_state,
            is_searching: false,
        })
    }

    pub fn set_migration_service(&mut self, service: Arc<MigrationService>) {
        self.migration_service = Some(service);
    }

    /// Refresh the tree - only load immediate children of root
    /// Subdirectories are loaded on-demand when expanded
    pub fn refresh_entries(&mut self) -> eyre::Result<()> {
        // Dispatch loading action
        if let Some(ref dispatcher) = self.dispatcher {
            dispatcher.dispatch(Action::EntriesLoading);
        }

        // Spawn async task to load ONLY root directory entries (non-recursive)
        let root_dir = self.base.clone();
        let filesystem = self.filesystem.clone();
        let dispatcher = self.dispatcher.clone();

        tokio::spawn(async move {
            // Only load immediate children, not recursive
            let result = filesystem.list_directory(&root_dir).await;

            let entries = match result {
                Ok(explorer_entries) => {
                    // Convert to ListEntry format
                    explorer_entries
                        .into_iter()
                        .map(|entry| {
                            let relative_path = entry
                                .path
                                .strip_prefix(&root_dir)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| entry.name.clone());

                            ListEntry {
                                name: entry.name.clone(),
                                relative_path,
                                selected: false,
                                is_directory: entry.is_directory,
                                status: EntryStatus::Unknown,
                            }
                        })
                        .collect()
                }
                Err(e) => {
                    log::error!("Failed to load root directory: {}", e);
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

    /// Load children of a specific directory (on-demand when expanded)
    fn load_directory_children(&mut self, dir_path: &str) -> eyre::Result<()> {
        let full_path = self.base.join(dir_path);
        let root_dir = self.base.clone();
        let filesystem = self.filesystem.clone();
        let dispatcher = self.dispatcher.clone();
        let dir_path_owned = dir_path.to_string();

        tokio::spawn(async move {
            let result = filesystem.list_directory(&full_path).await;

            let entries = match result {
                Ok(explorer_entries) => {
                    // Convert to ListEntry format
                    explorer_entries
                        .into_iter()
                        .map(|entry| {
                            let relative_path = entry
                                .path
                                .strip_prefix(&root_dir)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| entry.name.clone());

                            ListEntry {
                                name: entry.name.clone(),
                                relative_path,
                                selected: false,
                                is_directory: entry.is_directory,
                                status: EntryStatus::Unknown,
                            }
                        })
                        .collect()
                }
                Err(e) => {
                    log::error!("Failed to load directory {}: {}", dir_path_owned, e);
                    Vec::new()
                }
            };

            // Send results back via action with parent path
            if let Some(dispatcher) = dispatcher {
                dispatcher.dispatch(Action::DirectoryChildrenLoaded(dir_path_owned, entries));
            }
        });

        Ok(())
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
        // Get the selected node before toggling
        let selected_node = self.tree_state.selected_node();

        if let Some(node) = selected_node {
            if node.entry.is_directory {
                let dir_path = node.entry.relative_path.clone();
                let was_expanded = node.expanded;
                let has_children = node.has_children;

                // Toggle expansion state
                if self.tree_state.toggle_current_expansion() {
                    // Update widget state to reflect new visible rows
                    self.widget_state.select(Some(self.tree_state.cursor()));

                    // Load children if directory was just expanded and has no children yet
                    // (only load when expanding, not when collapsing)
                    if !was_expanded && !has_children {
                        self.load_directory_children(&dir_path)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Expand current directory (right arrow) - only expands, doesn't toggle
    pub fn expand_current_directory(&mut self) -> eyre::Result<()> {
        let (success, needs_load) = self.tree_state.expand_current();

        if success {
            self.widget_state.select(Some(self.tree_state.cursor()));

            // Load children if needed
            if needs_load {
                if let Some(node) = self.tree_state.selected_node() {
                    let dir_path = node.entry.relative_path.clone();
                    self.load_directory_children(&dir_path)?;
                }
            }
        }

        Ok(())
    }

    /// Collapse current folder or move to parent (left arrow)
    pub fn collapse_current_or_goto_parent(&mut self) -> eyre::Result<()> {
        let (collapsed, parent_index) = self.tree_state.collapse_current_or_goto_parent();

        if collapsed {
            // Successfully collapsed the current directory
            self.widget_state.select(Some(self.tree_state.cursor()));
        } else if let Some(parent_idx) = parent_index {
            // Move to parent
            self.tree_state.set_cursor(parent_idx);
            self.widget_state.select(Some(parent_idx));
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
            // Spawn async task to get directory children (recursively)
            let repo_base = self.base.clone();
            let rel_path = repo_base.join(&entry.relative_path);
            let root_dir = self.base.clone();
            let filesystem = self.filesystem.clone();
            let script_memory = self.script_memory.clone();
            let dispatcher = self.dispatcher.clone();
            let entry_path = entry.relative_path.clone();

            tokio::spawn(async move {
                match filesystem.list_sql_files_recursive(&rel_path).await {
                    Ok(paths) => {
                        // Filter out skipped scripts
                        let mut items: Vec<String> = Vec::new();
                        for path in paths {
                            if let Ok(relative_path) = path.strip_prefix(&root_dir) {
                                let path_str = relative_path.to_string_lossy().to_string();

                                // Check if script is skipped
                                let is_skipped = script_memory
                                    .get_script_record(&path_str)
                                    .ok()
                                    .flatten()
                                    .map(|rec| {
                                        rec.result == crate::script_memory::ScriptResult::Skipped
                                    })
                                    .unwrap_or(false);

                                if !is_skipped {
                                    items.push(path_str);
                                }
                            }
                        }

                        if let Some(dispatcher) = dispatcher {
                            dispatcher.dispatch(Action::ToggleSelection(items));
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to get children recursively for directory {}: {}",
                            entry_path,
                            e
                        );
                    }
                }
            });
        } else {
            // For single files, only toggle if not skipped
            if entry.status != EntryStatus::Skipped {
                state.toggle(entry.relative_path);
            }
        }
    }

    pub fn unselect_all(&mut self, state: &mut AppState) {
        state.selected.clear()
    }

    pub fn jump_to_next_not_run(&mut self) {
        // Optimized batch search with progress indicator
        let root_dir = self.base.clone();
        let filesystem = self.filesystem.clone();
        let script_memory = self.script_memory.clone();
        let dispatcher = self.dispatcher.clone();

        let current_path = self
            .tree_state
            .selected_node()
            .map(|n| n.entry.relative_path);

        // Show searching indicator
        if let Some(ref disp) = dispatcher {
            disp.dispatch(Action::SearchingForNextNotRun(true));
        }

        // Spawn async search
        tokio::spawn(async move {
            log::info!("Starting search for next Not Run script...");

            // Step 1: Get all executed scripts from database in ONE query
            log::info!("Querying database for executed scripts...");
            let executed_scripts = match script_memory.get_all_executed_scripts() {
                Ok(set) => {
                    log::info!("Found {} executed scripts in database", set.len());
                    set
                }
                Err(e) => {
                    log::error!("Failed to query executed scripts: {}", e);
                    if let Some(ref disp) = dispatcher {
                        disp.dispatch(Action::SearchingForNextNotRun(false));
                    }
                    return;
                }
            };

            // Step 2: Get all SQL files from filesystem
            log::info!(
                "Scanning filesystem for SQL files at {}...",
                root_dir.display()
            );
            let paths = match filesystem.list_sql_files_recursive(&root_dir).await {
                Ok(p) => {
                    log::info!("Found {} total SQL files", p.len());
                    p
                }
                Err(e) => {
                    log::error!("Failed to list files: {}", e);
                    if let Some(ref disp) = dispatcher {
                        disp.dispatch(Action::SearchingForNextNotRun(false));
                    }
                    return;
                }
            };

            // Step 3: Build list of Not Run files (not in database)
            log::info!("Filtering for Not Run scripts...");
            let mut not_run_files: Vec<String> = Vec::new();
            for path in paths {
                if let Ok(relative_path) = path.strip_prefix(&root_dir) {
                    let path_str = relative_path.to_string_lossy().to_string();
                    if !executed_scripts.contains(&path_str) {
                        not_run_files.push(path_str);
                    }
                }
            }

            log::info!("Found {} Not Run scripts", not_run_files.len());

            if not_run_files.is_empty() {
                log::warn!("No Not Run scripts found");
                if let Some(ref disp) = dispatcher {
                    disp.dispatch(Action::SearchingForNextNotRun(false));
                }
                return;
            }

            // Step 4: Find current position and search forward
            let current_index = if let Some(ref path) = current_path {
                let idx = not_run_files.iter().position(|p| p == path);
                log::info!("Current path: {:?}, index in Not Run list: {:?}", path, idx);
                idx
            } else {
                log::info!("No current path selected");
                None
            };

            let start_index = current_index.map(|i| i + 1).unwrap_or(0);
            log::info!("Starting search from index {}", start_index);

            // Search forward from current position
            if start_index < not_run_files.len() {
                let path = &not_run_files[start_index];
                log::info!("Found next Not Run script: {}", path);
                if let Some(ref disp) = dispatcher {
                    disp.dispatch(Action::JumpToPath(path.clone(), 0));
                    disp.dispatch(Action::SearchingForNextNotRun(false));
                }
                return;
            }

            // Wrap around to the beginning
            if !not_run_files.is_empty() {
                let path = &not_run_files[0];
                log::info!("Wrapping around to first Not Run script: {}", path);
                if let Some(ref disp) = dispatcher {
                    disp.dispatch(Action::JumpToPath(path.clone(), 0));
                    disp.dispatch(Action::SearchingForNextNotRun(false));
                }
                return;
            }

            // No Not Run script found (shouldn't happen since we checked empty above)
            log::warn!("Unexpected: checked for empty but no scripts to jump to");
            if let Some(ref disp) = dispatcher {
                disp.dispatch(Action::SearchingForNextNotRun(false));
            }
        });
    }

    pub fn select_from_cursor_to_end(&mut self, state: &mut AppState) {
        // OPTIMIZATION: Scan filesystem ONCE at the start, then filter in memory
        let flattened = self.tree_state.flattened().to_vec();
        let current_cursor = self.tree_state.cursor();

        let root_dir = self.base.clone();
        let script_memory = self.script_memory.clone();
        let dispatcher = self.dispatcher.clone();
        let filesystem = self.filesystem.clone();

        tokio::spawn(async move {
            // Step 1: Get ALL SQL files in repository ONCE (operation-scoped cache)
            log::debug!("Scanning filesystem once for select_from_cursor_to_end");
            let all_files = match filesystem.list_sql_files_recursive(&root_dir).await {
                Ok(files) => files,
                Err(e) => {
                    log::error!("Failed to scan filesystem: {}", e);
                    return;
                }
            };

            // Step 2: Convert to relative paths and build a HashSet for O(1) lookup
            let all_relative_paths: std::collections::HashSet<String> = all_files
                .iter()
                .filter_map(|p| {
                    p.strip_prefix(&root_dir)
                        .ok()
                        .map(|rp| rp.to_string_lossy().to_string())
                })
                .collect();

            log::debug!("Found {} total SQL files", all_relative_paths.len());

            // Step 3: Process entries from cursor to end using cached file list
            let mut items: Vec<String> = Vec::new();

            for node in flattened.iter().skip(current_cursor) {
                if node.entry.is_directory {
                    // Filter cached files that belong to this directory
                    let dir_prefix = if node.entry.relative_path.is_empty() {
                        String::new()
                    } else {
                        format!("{}/", node.entry.relative_path)
                    };

                    for file_path in &all_relative_paths {
                        // Check if file is in this directory (or subdirectories)
                        if file_path.starts_with(&dir_prefix) || dir_prefix.is_empty() {
                            // Check if script is skipped
                            let is_skipped = script_memory
                                .get_script_record(file_path)
                                .ok()
                                .flatten()
                                .map(|rec| {
                                    rec.result == crate::script_memory::ScriptResult::Skipped
                                })
                                .unwrap_or(false);

                            if !is_skipped && !items.contains(file_path) {
                                items.push(file_path.clone());
                            }
                        }
                    }
                } else {
                    // It's a file - check if it exists in our cached list
                    let path_str = &node.entry.relative_path;
                    if all_relative_paths.contains(path_str) {
                        // Check if script is skipped
                        let is_skipped = script_memory
                            .get_script_record(path_str)
                            .ok()
                            .flatten()
                            .map(|rec| rec.result == crate::script_memory::ScriptResult::Skipped)
                            .unwrap_or(false);

                        if !is_skipped {
                            items.push(path_str.clone());
                        }
                    }
                }
            }

            log::debug!("Selected {} files from cursor to end", items.len());

            if let Some(dispatcher) = dispatcher {
                dispatcher.dispatch(Action::ToggleSelection(items));
            }
        });
    }

    pub fn toggle_skip(&mut self) {
        let entry = self.get_selection();

        if entry.is_none() {
            return;
        }

        let entry = entry.unwrap();
        let command_tx = self.command_tx.clone();

        if entry.is_directory {
            // Toggle skip for all files in directory (recursively)
            // For directories, we need to check if any file inside is currently skipped
            // to determine whether to skip or unskip the whole directory
            let repo_base = self.base.clone();
            let rel_path = repo_base.join(&entry.relative_path);
            let root_dir = self.base.clone();
            let filesystem = self.filesystem.clone();
            let script_memory_check = self.script_memory.clone();
            let script_memory = self.script_memory.clone();
            let entry_path = entry.relative_path.clone();

            tokio::spawn(async move {
                match filesystem.list_sql_files_recursive(&rel_path).await {
                    Ok(paths) => {
                        if paths.is_empty() {
                            return;
                        }

                        // Check the first file to determine if directory is currently skipped
                        let first_file_path = paths[0]
                            .strip_prefix(&root_dir)
                            .ok()
                            .map(|p| p.to_string_lossy().to_string());

                        let is_currently_skipped = if let Some(ref first_path) = first_file_path {
                            script_memory_check
                                .get_script_record(first_path)
                                .ok()
                                .flatten()
                                .map(|rec| {
                                    rec.result == crate::script_memory::ScriptResult::Skipped
                                })
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        // Now toggle all files in the directory
                        for path in paths {
                            if let Ok(relative_path) = path.strip_prefix(&root_dir) {
                                let path_str = relative_path.to_string_lossy().to_string();

                                // Toggle: if currently skipped, unmark; otherwise mark as skipped
                                let result = if is_currently_skipped {
                                    script_memory.unmark_skipped(path_str.clone())
                                } else {
                                    script_memory.mark_skipped(path_str.clone())
                                };

                                if let Err(e) = result {
                                    log::error!("Failed to toggle skip status: {}", e);
                                } else {
                                    // Dispatch status change for immediate UI update
                                    let new_status = if is_currently_skipped {
                                        EntryStatus::NeverStarted
                                    } else {
                                        EntryStatus::Skipped
                                    };
                                    if let Some(ref tx) = command_tx {
                                        let _ = tx
                                            .send(Action::EntryStatusChanged(path_str, new_status));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to get children recursively for directory {}: {}",
                            entry_path,
                            e
                        );
                    }
                }
            });
        } else {
            // Toggle skip for single file
            let path = entry.relative_path.clone();
            let script_memory = self.script_memory.clone();
            let is_currently_skipped = entry.status == EntryStatus::Skipped;

            tokio::spawn(async move {
                let result = if is_currently_skipped {
                    script_memory.unmark_skipped(path.clone())
                } else {
                    script_memory.mark_skipped(path.clone())
                };

                if let Err(e) = result {
                    log::error!("Failed to toggle skip status: {}", e);
                } else {
                    // Dispatch status change for immediate UI update
                    let new_status = if is_currently_skipped {
                        EntryStatus::NeverStarted
                    } else {
                        EntryStatus::Skipped
                    };
                    if let Some(ref tx) = command_tx {
                        let _ = tx.send(Action::EntryStatusChanged(path, new_status));
                    }
                }
            });
        }
    }

    /// Get the currently highlighted script for the preview panel
    fn get_highlighted_script(&mut self, state: &AppState) -> Option<Action> {
        use crate::app::{Script, ScriptState};
        use crate::entries::EntryStatus;

        // Get the currently selected entry
        let entry = self.get_selection()?;

        // Only highlight files, not directories
        if entry.is_directory {
            return Some(Action::ScriptHighlighted(None));
        }

        // Check if script is in the selected/executed list (current session)
        let script = if let Some(existing) = state
            .selected
            .iter()
            .find(|s| s.relative_path == entry.relative_path)
        {
            // Use existing script with execution state from current session
            existing.clone()
        } else {
            // Map the entry status (from database/checksum) to script state
            let script_state = match entry.status {
                EntryStatus::Finished(true) => ScriptState::Finished,
                EntryStatus::Finished(false) => ScriptState::Error,
                _ => ScriptState::None,
            };

            // Create new script entry for preview with persisted state
            Script {
                relative_path: entry.relative_path.clone(),
                state: script_state,
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
            Action::DirectoryExpand => {
                self.expand_current_directory()?;
                return Ok(None);
            }
            Action::DirectoryCollapse => {
                self.collapse_current_or_goto_parent()?;
                return Ok(None);
            }
            Action::SelectCurrent => {
                self.select_current(state);
                return Ok(None);
            }
            Action::UnselectAll => {
                self.unselect_all(state);
                return Ok(None);
            }
            Action::ToggleSkip => {
                self.toggle_skip();
                return Ok(None);
            }
            Action::JumpToNextNotRun => {
                self.jump_to_next_not_run();
                return Ok(None);
            }
            Action::JumpToPath(path, retry_count) => {
                const MAX_RETRIES: usize = 10;

                // First, ensure all parent directories are loaded into the tree
                let path_parts: Vec<&str> = path.split('/').collect();

                // Load all parent directories if they haven't been loaded yet
                for i in 1..path_parts.len() {
                    let parent_path = path_parts[..i].join("/");

                    // Only load if this directory doesn't have children yet
                    if !self.tree_state.has_children_loaded(&parent_path) {
                        log::info!("Loading parent directory: {}", parent_path);
                        if let Err(e) = self.load_directory_children(&parent_path) {
                            log::debug!("Failed to load parent directory {}: {}", parent_path, e);
                        }
                    } else {
                        log::debug!(
                            "Parent directory {} already has children loaded",
                            parent_path
                        );
                    }
                }

                // Now try to expand and find the path
                if let Some(index) = self.tree_state.expand_and_find_path(&path) {
                    log::info!("Successfully jumped to path: {}", path);
                    self.tree_state.set_cursor(index);
                    self.widget_state.select(Some(index));
                    return Ok(Some(Action::Render));
                } else if retry_count < MAX_RETRIES {
                    // Still not found - retry after a short delay to let async loading complete
                    log::info!(
                        "Path {} not in tree yet (retry {}), waiting for async load...",
                        path,
                        retry_count
                    );
                    if let Some(ref disp) = self.dispatcher {
                        let path_clone = path.clone();
                        let disp_clone = disp.clone();
                        let next_retry = retry_count + 1;
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                            disp_clone.dispatch(Action::JumpToPath(path_clone, next_retry));
                        });
                    }
                } else {
                    log::error!("Failed to jump to path {} after {} retries - directory may not exist in tree", path, MAX_RETRIES);
                }
                return Ok(None);
            }
            Action::SelectFromCursorToEnd => {
                self.select_from_cursor_to_end(state);
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
                    (&self.migration_service, &self.dispatcher)
                {
                    use crate::domain::ScriptPath;

                    // Convert entries to ScriptPaths (only files, not directories)
                    let script_paths: Vec<ScriptPath> = entries
                        .iter()
                        .filter(|e| !e.is_directory)
                        .filter_map(|e| ScriptPath::new(e.relative_path.clone()).ok())
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
                    // Legacy fallback (old implementation) - now also database-only
                    log::warn!("MigrationService not available, using legacy status calculation");
                    let channel: Option<UnboundedSender<Action>> = self.command_tx.clone();
                    let memory = self.script_memory.clone();
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

                            // Get database-only status (no file reading, no CRC)
                            match memory.get_script_record(&entry.relative_path) {
                                Ok(Some(record)) => {
                                    // Convert ScriptResult to EntryStatus (using From trait)
                                    let status = EntryStatus::from(record.result);
                                    send_through_channel(
                                        &channel,
                                        Action::EntryStatusChanged(entry.relative_path, status),
                                    );
                                }
                                Ok(None) => {
                                    // Never executed
                                    send_through_channel(
                                        &channel,
                                        Action::EntryStatusChanged(
                                            entry.relative_path,
                                            EntryStatus::NeverStarted,
                                        ),
                                    );
                                }
                                Err(e) => {
                                    log::error!(
                                        "Error getting status for {} : {}",
                                        entry.relative_path,
                                        e
                                    );
                                }
                            }
                        }
                    });
                }

                return Ok(None);
            }
            Action::CheckForChanges => {
                // Check for file modifications (CRC check)
                // Get all entries from tree
                let entries = self.tree_state.entries();

                // Use MigrationService if available
                if let (Some(migration_service), Some(dispatcher)) =
                    (&self.migration_service, &self.dispatcher)
                {
                    use crate::domain::ScriptPath;

                    // Convert entries to ScriptPaths (only files, not directories)
                    let script_paths: Vec<ScriptPath> = entries
                        .iter()
                        .filter(|e| !e.is_directory)
                        .filter_map(|e| ScriptPath::new(e.relative_path.clone()).ok())
                        .collect();

                    // Use service to check for changes asynchronously
                    migration_service.check_for_changes(script_paths, dispatcher);
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
            Action::DirectoryChildrenLoaded(parent_path, children) => {
                // Calculate statuses for ONLY the new children (before adding to tree)
                if let (Some(migration_service), Some(dispatcher)) =
                    (&self.migration_service, &self.dispatcher)
                {
                    use crate::domain::ScriptPath;

                    // Dispatch directory statuses immediately
                    for child in &children {
                        if child.is_directory {
                            dispatcher.dispatch(Action::EntryStatusChanged(
                                child.relative_path.clone(),
                                EntryStatus::Directory,
                            ));
                        }
                    }

                    // Convert only the new file children to ScriptPaths
                    let script_paths: Vec<ScriptPath> = children
                        .iter()
                        .filter(|e| !e.is_directory)
                        .filter_map(|e| ScriptPath::new(e.relative_path.clone()).ok())
                        .collect();

                    // Calculate statuses only for the new children
                    if !script_paths.is_empty() {
                        migration_service.calculate_statuses(script_paths, dispatcher);
                    }
                }

                // Add children to the tree under the parent directory
                self.tree_state
                    .add_children_to_directory(&parent_path, children);

                // Sync widget state
                self.widget_state.select(Some(self.tree_state.cursor()));

                return Ok(None);
            }
            Action::StatusCalculationProgress(current, total) => {
                // Store progress for rendering (will add field to List struct)
                // For now, just trigger render
                // TODO: Add crc_progress field to List
                let _ = (current, total); // Suppress unused warning

                // Request a render to show the progress update
                return Ok(Some(Action::Render));
            }
            Action::SearchingForNextNotRun(is_searching) => {
                self.is_searching = is_searching;
                return Ok(Some(Action::Render));
            }
            Action::AddSelection(paths) => {
                state.add_many(&paths);
                return Ok(Some(Action::Render));
            }
            Action::RemoveSelection(paths) => {
                state.remove_many(&paths);
                return Ok(Some(Action::Render));
            }
            Action::ToggleSelection(paths) => {
                state.toggle_many(&paths);
                return Ok(Some(Action::Render));
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
                    EntryStatus::NeverStarted => ("\u{2022}", Style::new().fg(Color::Cyan)),
                    // ⊗ Skipped (marked to never run)
                    EntryStatus::Skipped => ("\u{2297}", Style::new().fg(Color::DarkGray)),
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

        let title = if self.is_searching {
            "Searching for next Not Run..."
        } else {
            "Press h for help"
        };

        let list_draw = ratatui::widgets::List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title_position(Position::Bottom)
                    .title_alignment(Alignment::Right)
                    .title(title),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true);

        f.render_widget(path_draw, rects[0]);
        f.render_stateful_widget(list_draw, rects[1], &mut self.widget_state);

        Ok(())
    }
}
