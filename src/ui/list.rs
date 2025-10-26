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
    infrastructure::FilesystemRepository,
    domain::MigrationRepository,
};
use crate::{app::AppState, entries::ListEntry};
use std::sync::Arc;

pub struct List {
    base: PathBuf,
    command_tx: Option<UnboundedSender<Action>>,
    dispatcher: Option<ActionDispatcher>,
    migration_service: Option<Arc<MigrationService>>,
    config: Settings,
    state: ListState,
    repository: FilesystemRepository,  // Owned, not Arc - List needs mutable access for navigation
    entries: Vec<ListEntry>,
    script_memory: ScriptDatabase,
}

impl List {
    pub fn new(
        repository: FilesystemRepository,
        base: PathBuf,
        script_memory: ScriptDatabase,
    ) -> Result<Self> {
        // Build initial entries - we'll populate them after construction
        Ok(Self {
            state: ListState::default().with_selected(Some(0)),
            command_tx: None,
            dispatcher: None,
            migration_service: None,
            config: Settings::default(),
            entries: Vec::new(),  // Will be populated via refresh_entries()
            script_memory,
            repository,
            base,
        })
    }

    pub fn set_migration_service(&mut self, service: Arc<MigrationService>) {
        self.migration_service = Some(service);
    }

    /// Refresh the entries list from the current directory
    pub fn refresh_entries(&mut self) -> eyre::Result<()> {
        use std::fs;

        let current_dir = self.repository.current_directory().to_path_buf();
        let root_dir = self.repository.root_directory().to_path_buf();

        let mut entries = Vec::new();

        // Use repository trait to get SQL scripts
        let handle = tokio::runtime::Handle::current();
        let scripts = handle.block_on(async {
            self.repository.list_scripts(&current_dir).await
        });

        match scripts {
            Ok(script_paths) => {
                // Add script files
                for script_path in script_paths {
                    if let (Some(name), Some(path_str)) = (
                        script_path.as_path().file_name().and_then(|n| n.to_str()),
                        script_path.as_str()
                    ) {
                        entries.push(ListEntry {
                            name: name.to_string(),
                            relative_path: path_str.to_string(),
                            selected: false,
                            is_directory: false,
                            status: EntryStatus::Unknown,
                        });
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to list scripts: {}", e);
            }
        }

        // Also need to list directories (repository trait is for scripts only)
        for entry in fs::read_dir(&current_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                let file_name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden directories
                if file_name.starts_with('.') || file_name.starts_with('_') {
                    continue;
                }

                let relative_path = match entry.path().strip_prefix(&root_dir) {
                    Ok(rel) => rel.to_string_lossy().to_string(),
                    Err(_) => file_name.clone(),
                };

                entries.push(ListEntry {
                    name: file_name,
                    relative_path,
                    selected: false,
                    is_directory: true,
                    status: EntryStatus::Unknown,
                });
            }
        }

        // Sort entries: directories first, then by name
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        self.entries = entries;
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
            self.repository.enter_directory(&name);
            self.refresh_entries()?;

            if let Some(ref dispatcher) = self.dispatcher {
                dispatcher.dispatch(Action::CalculateEntryStatus);
            }

            if !self.entries.is_empty() {
                self.state.select(Some(0))
            } else {
                self.state.select(None)
            }
        }

        Ok(())
    }

    pub fn leave_current_directory(&mut self) -> eyre::Result<()> {
        let old_dir = self.repository.leave_directory();
        if let Some(old_dir) = old_dir {
            self.refresh_entries()?;
            self.state.select(Some(0));

            if let Some(ref dispatcher) = self.dispatcher {
                dispatcher.dispatch(Action::CalculateEntryStatus);
            }

            let old_index = self.entries.iter().position(|r| r.name == old_dir);

            if let Some(old_index) = old_index {
                self.state.select(Some(old_index));
            } else if !self.entries.is_empty() {
                self.state.select(Some(0))
            } else {
                self.state.select(None)
            }
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
            // Use async task to get children via trait method
            let repo_base = self.repository.root_directory().to_path_buf();
            let rel_path = repo_base.join(&entry.relative_path);

            // For now, we need blocking behavior in this sync context
            // Spawn a task and block on it
            let handle = tokio::runtime::Handle::current();
            let items = handle.block_on(async {
                match self.repository.get_children(&rel_path).await {
                    Ok(script_paths) => script_paths.into_iter()
                        .filter_map(|sp| sp.as_str().map(|s| s.to_string()))
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
            let repo_base = self.repository.root_directory().to_path_buf();
            let rel_path = repo_base.join(&entry.relative_path);

            let handle = tokio::runtime::Handle::current();
            let items = handle.block_on(async {
                match self.repository.get_children(&rel_path).await {
                    Ok(script_paths) => script_paths.into_iter()
                        .filter_map(|sp| sp.as_str().map(|s| s.to_string()))
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

        // Use async task to get scripts after (globally from repo root)
        let handle = tokio::runtime::Handle::current();
        let entries = handle.block_on(async {
            match self.repository.get_scripts_after_global(&entry.name).await {
                Ok(script_paths) => script_paths.into_iter()
                    .filter_map(|sp| sp.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>(),
                Err(e) => {
                    log::error!("Failed to get scripts after {}: {}", entry.name, e);
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

        // Use async task to get scripts after in current directory
        let handle = tokio::runtime::Handle::current();
        let entries = handle.block_on(async {
            match self.repository.get_scripts_after_in_current(&entry.name).await {
                Ok(script_paths) => script_paths.into_iter()
                    .filter_map(|sp| sp.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>(),
                Err(e) => {
                    log::error!("Failed to get scripts after {} in directory: {}", entry.name, e);
                    vec![]
                }
            }
        });

        state.add_many(&entries);
    }

    pub fn select_all_in_directory(&mut self, state: &mut AppState) {
        // Use async task to get all scripts in current directory
        let handle = tokio::runtime::Handle::current();
        let entries = handle.block_on(async {
            match self.repository.get_scripts_in_current().await {
                Ok(script_paths) => script_paths.into_iter()
                    .filter_map(|sp| sp.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>(),
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
        dispatcher.dispatch(Action::CalculateEntryStatus);
        self.dispatcher = Some(dispatcher);
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Settings) -> Result<()> {
        self.config = config;
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
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &AppState) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        let path_span = Span::raw(
            self.repository
                .current_directory()
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
        Ok(())
    }
}
