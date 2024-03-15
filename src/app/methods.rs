use std::path::Path;

use crate::{
    entries::{Entry, Name},
    read_entries,
};

use super::{App, Message, Screen};

impl App {
    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.exit = true;
    }

    pub fn cursor_up(&mut self) {
        if let Some(position) = self.ui_state.list.selected() {
            if position > 0 {
                self.ui_state.list.select(Some(position - 1))
            }
        }
    }

    pub fn cursor_down(&mut self, entries_len: usize) {
        if let Some(position) = self.ui_state.list.selected() {
            if position < entries_len - 1 {
                self.ui_state.list.select(Some(position + 1))
            }
        }
    }

    pub fn go_to_top(&mut self) {
        self.ui_state.list.select(Some(0));
    }

    pub fn go_to_bottom(&mut self, entries_len: usize) {
        self.ui_state.list.select(Some(entries_len - 1));
    }

    pub fn open_selected_directory(&mut self) {
        match &mut self.current_screen {
            Screen::FileChooser { entries } => {
                if let Some(selected) = self.ui_state.list.selected() {
                    let dir_name = entries.get(selected);
                    if let Some(entry) = dir_name {
                        match entry {
                            Entry::Directory(dir_name) => {
                                let new_path = self.base_path.join(std::path::Path::new(&dir_name));
                                self.base_path = new_path;
                                *entries = read_entries(&self.base_path);

                                if entries.len() > 0 {
                                    self.ui_state.list.select(Some(0))
                                } else {
                                    self.ui_state.list.select(None)
                                }
                            }
                            Entry::File(_) => {}
                        }
                    }
                }
            }
        }
    }

    pub fn leave_directory(&mut self) {
        match &mut self.current_screen {
            Screen::FileChooser { entries } => {
                let path = self.base_path.clone();
                let old_path = path.as_path();
                if let (Some(new_path), Some(old_dir)) = (old_path.parent(), old_path.file_name()) {
                    self.base_path = new_path.to_path_buf();
                    *entries = read_entries(&self.base_path);
                    self.ui_state.list.select(Some(0));

                    let old_index = entries
                        .iter()
                        .position(|r| r.get_name() == old_dir.to_str().unwrap());

                    if let Some(old_index) = old_index {
                        self.ui_state.list.select(Some(old_index));
                    } else {
                        if entries.len() > 0 {
                            self.ui_state.list.select(Some(0))
                        } else {
                            self.ui_state.list.select(None)
                        }
                    }
                }
            }
        }
    }

    pub async fn execute_selected_script(&mut self) {
        match &self.current_screen {
            Screen::FileChooser { entries } => {
                if let Some(selected) = self.ui_state.list.selected() {
                    if let Some(entry) = entries.get(selected) {
                        if let Entry::File(file) = entry {
                            let full_path = self.base_path.join(Path::new(&file));
                            match self.connection.execute_script(full_path).await {
                                Err(e) => self.message = Some(Message::Error(e.to_string())),
                                _ => {
                                    self.message =
                                        Some(Message::Success("Script execution done".into()))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
