use color_eyre::eyre::{Ok, Result};

use ratatui::{
    prelude::*,
    widgets::{block::Position, *},
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Settings, repository::Repository, tui::Frame};
use crate::{app::AppState, entries::ListEntry};
pub struct List {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    state: ListState,
    repository: Repository,
    entries: Vec<ListEntry>,
}

impl List {
    pub fn new(repository: Repository) -> Self {
        Self {
            state: ListState::default().with_selected(Some(0)),
            command_tx: None,
            config: Settings::default(),
            entries: repository.read_entries_in_current_directory(),
            repository,
        }
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

    pub fn open_selected_directory(&mut self) {
        let entry = self.get_selection().cloned();

        if let Some(ListEntry {
            is_directory: true,
            name,
            ..
        }) = entry
        {
            self.repository.open_directory(&name);
            self.entries = self.repository.read_entries_in_current_directory();

            if !self.entries.is_empty() {
                self.state.select(Some(0))
            } else {
                self.state.select(None)
            }
        }
    }
    pub fn leave_current_directory(&mut self) {
        let old_dir = self.repository.leave_directory();
        if let Some(old_dir) = old_dir {
            self.entries = self.repository.read_entries_in_current_directory();
            self.state.select(Some(0));

            let old_index = self.entries.iter().position(|r| r.name == old_dir);

            if let Some(old_index) = old_index {
                self.state.select(Some(old_index));
            } else if !self.entries.is_empty() {
                self.state.select(Some(0))
            } else {
                self.state.select(None)
            }
        }
    }

    pub fn select_current(&mut self, state: &mut AppState) {
        let entry = self.get_selection().cloned();

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        if entry.is_directory {
            let items = self.repository.get_children(entry.relative_path);
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
            let items = self.repository.get_children(entry.relative_path);
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

        let entries = self
            .repository
            .read_files_after_in_directory(&entry.name)
            .unwrap_or_default();

        state.add_many(&entries);
    }

    pub fn select_all_in_directory(&mut self, state: &mut AppState) {
        let entries = self
            .repository
            .read_files_in_directory()
            .unwrap_or_default();

        state.add_many(&entries);
    }
}

impl Component for List {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
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
                self.open_selected_directory();
                return Ok(None);
            }
            Action::DirectoryLeave => {
                self.leave_current_directory();
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
            Action::SelectAllInDirectory => {
                self.select_all_in_directory(state);
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
                .current_as_path_buf()
                .as_path()
                .display()
                .to_string(),
        );
        let path_draw = Line::default().spans(vec![path_span]);

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let name = entry.name.clone();
                let selected = state
                    .selected
                    .iter()
                    .any(|s| s.relative_path == entry.relative_path);
                let style = match (selected, entry.is_directory) {
                    (true, false) => Style::new().green(),
                    (false, false) => Style::new().white(),
                    (_, true) => Style::new().light_blue(),
                };

                let list_item = ListItem::new(name).style(style);
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
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true);

        f.render_widget(path_draw, rects[0]);
        f.render_stateful_widget(list_draw, rects[1], &mut self.state);
        Ok(())
    }
}
