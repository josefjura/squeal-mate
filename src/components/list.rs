use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    app::MessageType,
    db::Database,
    entries::{Entry, Name},
    read_entries,
    tui::Frame,
};

pub struct List {
    command_tx: Option<UnboundedSender<Action>>,
    config: HashMap<String, String>,
    state: ListState,
    entries: Vec<Entry>,
    base_path: PathBuf,
    connection: Database,
}

impl List {
    pub fn new(entries: Vec<Entry>, base_path: PathBuf, connection: Database) -> Self {
        Self {
            state: ListState::default().with_selected(Some(0)),
            entries,
            command_tx: None,
            config: HashMap::<String, String>::default(),
            base_path,
            connection,
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
                        let new_path = self.base_path.join(std::path::Path::new(&dir_name));
                        self.base_path = new_path;
                        self.entries = read_entries(&self.base_path);

                        if self.entries.len() > 0 {
                            self.state.select(Some(0))
                        } else {
                            self.state.select(None)
                        }
                    }
                    Entry::File(_) => {}
                }
            }
        }
    }
    pub fn leave_current_directory(&mut self) {
        let path = self.base_path.clone();
        let old_path = path.as_path();
        if let (Some(new_path), Some(old_dir)) = (old_path.parent(), old_path.file_name()) {
            self.base_path = new_path.to_path_buf();
            self.entries = read_entries(&self.base_path);
            self.state.select(Some(0));

            let old_index = self
                .entries
                .iter()
                .position(|r| r.get_name() == old_dir.to_str().unwrap());

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
    //TODO: Revive!!!
    // pub async fn execute_selected_script(&mut self) -> Option<Message> {
    //     if let Some(selected) = self.state.selected() {
    //         if let Some(entry) = self.entries.get(selected) {
    //             if let Entry::File(file) = entry {
    //                 let full_path = self.base_path.join(Path::new(&file));
    //                 return match self.connection.execute_script(full_path).await {
    //                     Err(e) => Some(Message::Error(e.to_string())),
    //                     _ => Some(Message::Success("Script execution done".into())),
    //                 };
    //             }
    //         }
    //     }

    //     None
    // }
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
            Action::ScriptRun => {
                if let Some(selected) = self.state.selected() {
                    if let Some(entry) = self.entries.get(selected) {
                        if let Entry::File(file) = entry {
                            let full_path = self.base_path.join(Path::new(&file));
                            let connection = self.connection.clone();
                            if let Some(channel) = &self.command_tx {
                                // TODO: Respond to error!
                                let _ = channel.send(Action::Message(
                                    "Executing script".into(),
                                    MessageType::Info,
                                ));
                            }

                            let channel = self.command_tx.clone();

                            tokio::spawn(async move {
                                if let Some(channel) = channel {
                                    let _ = channel.send(Action::StartSpinner);

                                    let result = connection.execute_script(full_path).await;

                                    match result {
                                        Ok(_) => {
                                            let _ = channel.send(Action::Message(
                                                "Finished execution".into(),
                                                MessageType::Success,
                                            ));
                                        }
                                        Err(err) => {
                                            let _ = channel.send(Action::Message(
                                                err.to_string(),
                                                MessageType::Error,
                                            ));
                                        }
                                    }

                                    let _ = channel.send(Action::StopSpinner);
                                }
                            });
                            return Ok(None);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
                Constraint::Length(1), // first row
            ])
            .split(area);

        let list_draw = ratatui::widgets::List::new(&self.entries)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double),
            )
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true);

        f.render_stateful_widget(list_draw, rects[0], &mut self.state);
        Ok(())
    }
}

impl<'a> From<&Entry> for ListItem<'a> {
    fn from(value: &Entry) -> Self {
        let style = match value {
            Entry::File(_) => Style::new().white(),
            Entry::Directory(_) => Style::new().blue(),
        };

        ListItem::<'a>::new(value.get_name().to_string()).style(style)
    }
}
