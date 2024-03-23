use std::{collections::HashMap, path::PathBuf};

use color_eyre::eyre::{Error, Result};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListState},
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    app::MessageType,
    db::Database,
    entries::{Entry, Name},
    tui::Frame,
};

pub struct ScrollList {
    command_tx: Option<UnboundedSender<Action>>,
    config: HashMap<String, String>,
    state: ListState,
    entries: Vec<Entry>,
    db: Database,
    base: PathBuf,
}

impl ScrollList {
    pub fn new(db: Database, base: PathBuf) -> Self {
        Self {
            command_tx: None,
            config: HashMap::<String, String>::default(),
            state: ListState::default().with_selected(Some(0)),
            entries: vec![],
            db,
            base,
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
}

impl Component for ScrollList {
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
            Action::RemoveSelectedScript => {
                if let Some(pos) = self.state.selected() {
                    let entry = self.entries.get(pos);
                    if let Some(entry) = entry {
                        return Ok(Some(Action::RemoveScript(entry.clone())));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn update_background(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::SelectScripts(scripts) => {
                self.entries.clear();
                self.entries.extend(scripts);
                self.entries.sort();
                return Ok(None);
            }
            Action::AppendScripts(scripts) => {
                let mut only_new: Vec<Entry> = scripts
                    .into_iter()
                    .filter(|s| !self.entries.contains(s))
                    .collect();
                self.entries.append(&mut only_new);
                self.entries.sort();
                return Ok(None);
            }
            Action::RemoveScript(entry) => self.entries.retain(|e| *e != entry),
            Action::RemoveAllSelectedScripts => self.entries.clear(),
            Action::ScriptRun => {
                let selected = self
                    .state
                    .selected()
                    .ok_or_else(|| Error::msg("No selection"))?;
                let entry = self
                    .entries
                    .get(selected)
                    .ok_or_else(|| Error::msg("Invalid entry"))?;

                if let Entry::File(_) = entry {
                    let full_path = self.base.join(entry.get_full_path()?);
                    let connection = self.db.clone();
                    send_through_channel(
                        &self.command_tx,
                        Action::Message("Executing script".into(), MessageType::Info),
                    );

                    let channel: Option<UnboundedSender<Action>> = self.command_tx.clone();

                    tokio::spawn(async move {
                        send_through_channel(&channel, Action::StartSpinner);

                        let result = connection.execute_script(full_path).await;

                        match result {
                            Ok(_) => {
                                send_through_channel(
                                    &channel,
                                    Action::Message(
                                        "Finished execution".into(),
                                        MessageType::Success,
                                    ),
                                );
                            }
                            Err(err) => {
                                send_through_channel(
                                    &channel,
                                    Action::Message(err.to_string(), MessageType::Error),
                                );
                            }
                        }

                        send_through_channel(&channel, Action::StopSpinner);
                    });
                    return Ok(None);
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let items: Vec<String> = self
            .entries
            .iter()
            .filter_map(|e| e.get_full_path().ok()?.to_str().map(str::to_owned))
            .map(String::from)
            .collect();

        let list_draw = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double),
            )
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true);

        f.render_stateful_widget(&list_draw, area, &mut self.state);

        Ok(())
    }
}

fn send_through_channel(channel: &Option<UnboundedSender<Action>>, action: Action) {
    if let Some(channel) = channel {
        if let Err(error) = channel.send(action) {
            log::error!("{}", error);
        }
    }
}
