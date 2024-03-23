use std::{collections::HashMap, path::Path, vec};

use color_eyre::eyre::{Error, Result};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    app::MessageType,
    db::Database,
    entries::{Entry, Name},
    repository::Repository,
    tui::Frame,
};

pub struct List {
    command_tx: Option<UnboundedSender<Action>>,
    config: HashMap<String, String>,
    state: ListState,
    repository: Repository,
    entries: Vec<Entry>,
    selection: Vec<Entry>,
}

impl List {
    pub fn new(repository: Repository) -> Self {
        Self {
            state: ListState::default().with_selected(Some(0)),
            command_tx: None,
            config: HashMap::<String, String>::default(),
            entries: repository.read_entries_in_current_directory(),
            repository,
            selection: vec![],
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

    pub fn open_selected_directory(&mut self) {
        if let Some(selected) = self.state.selected() {
            let dir_name = self.entries.get(selected);
            if let Some(entry) = dir_name {
                match entry {
                    Entry::Directory(dir_name) => {
                        self.repository.open_directory(&dir_name);
                        self.entries = self.repository.read_entries_in_current_directory();

                        if self.entries.len() > 0 {
                            self.state.select(Some(0))
                        } else {
                            self.state.select(None)
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    pub fn leave_current_directory(&mut self) {
        let old_dir = self.repository.leave_directory();
        if let Some(old_dir) = old_dir {
            self.entries = self.repository.read_entries_in_current_directory();
            self.state.select(Some(0));

            let old_index = self
                .entries
                .iter()
                .position(|r| r.get_filename_ref() == &old_dir);

            if let Some(old_index) = old_index {
                self.state.select(Some(old_index));
            } else {
                if self.entries.len() > 0 {
                    self.state.select(Some(0))
                } else {
                    self.state.select(None)
                }
            }
        }
    }
}

impl Component for List {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: HashMap<String, String>) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
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
            Action::RemoveSelectedScript => {
                if let Some(index) = self.state.selected() {
                    if let Some(entry) = self.entries.get(index) {
                        if self.selection.contains(entry) {
                            self.selection.retain(|e| *e != *entry);
                            return Ok(Some(Action::RemoveScript(entry.clone())));
                        }
                    }
                }
            }
            Action::SelectCurrent => {
                if let Some(index) = self.state.selected() {
                    if let Some(entry) = self.entries.get(index) {
                        if self.selection.contains(entry) {
                            self.selection.retain(|e| *e != *entry);
                            return Ok(Some(Action::RemoveScript(entry.clone())));
                        } else {
                            self.selection.push(entry.clone());
                            return Ok(Some(Action::AppendScripts(vec![entry.clone()])));
                        }
                    }
                }
            }
            Action::SelectAllAfter => {
                if let Some(index) = self.state.selected() {
                    let filename = self.entries.get(index);
                    if let Some(entry) = filename {
                        let entries = self.repository.read_files_after_in_directory(entry);
                        let result = entries.unwrap();
                        self.selection.extend(result.iter().cloned());
                        return Ok(Some(Action::AppendScripts(result)));
                    }
                }
            }
            Action::SelectAllInDirectory => {
                let entries = self.repository.read_files_in_directory();
                let result = entries.unwrap();
                // let mut selection: Vec<String> = result
                //     .iter()
                //     .filter_map(|f| f.as_path().to_str())
                //     .map(|f| String::from(f))
                //     .collect();

                self.selection.extend(result.iter().cloned());
                return Ok(Some(Action::AppendScripts(result)));
            }
            _ => {}
        }
        Ok(None)
    }

    fn update_background(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::RemoveAllSelectedScripts => self.selection.clear(),
            Action::RemoveScript(entry) => self.selection.retain(|e| *e != entry),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1), // first row
            ])
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
            .filter_map(|entry| {
                let name = entry.get_filename_ref().clone();
                let style = match entry {
                    Entry::File(_) => {
                        if self.selection.contains(entry) {
                            Style::new().white().on_light_green()
                        } else {
                            Style::new().white()
                        }
                    }
                    Entry::Directory(_) => Style::new().light_blue(),
                };

                let list_item = ListItem::new(name).style(style);
                Some(list_item)
            })
            .collect();

        let list_draw = ratatui::widgets::List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double),
            )
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true);

        f.render_widget(path_draw, rects[0]);
        f.render_stateful_widget(list_draw, rects[1], &mut self.state);
        Ok(())
    }
}
