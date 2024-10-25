use std::path::PathBuf;

use color_eyre::eyre::Result;
use crc::{Crc, CRC_32_ISO_HDLC};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};
use tokio::{sync::mpsc::UnboundedSender, time::Instant};

use super::Component;
use crate::{
    action::Action,
    app::{AppState, Script, ScriptState},
    config::Settings,
    db::Database,
    script_memory::ScriptDatabase,
    tui::Frame,
    utils::send_through_channel,
};

pub struct ScrollList {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,
    state: ListState,
    db: Database,
    base: PathBuf,
    script_memory: ScriptDatabase,
}

impl ScrollList {
    pub fn new(db: Database, base: PathBuf, script_memory: ScriptDatabase) -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            state: ListState::default().with_selected(Some(0)),
            db,
            base,
            script_memory,
        }
    }

    fn update_selection(&mut self, state: &mut AppState) {
        if self.command_tx.is_none() {
            return;
        }

        if let Some(channel) = self.command_tx.as_mut() {
            let selected: Vec<String> = state
                .selected
                .iter()
                .map(|e| e.relative_path.clone())
                .collect();

            if selected.is_empty() {
                return;
            }

            if let Err(error) = channel.send(Action::SelectionChanged(selected)) {
                log::error!("{}", error);
            }
        }
    }

    fn get_update(&self, state: &mut AppState) -> Result<Option<Action>> {
        if let Some(pos) = self.state.selected() {
            let entry = state.selected.get(pos);
            if let Some(entry) = entry {
                return Ok(Some(Action::ScriptHighlighted(Some(entry.clone()))));
            } else {
                return Ok(Some(Action::ScriptHighlighted(None)));
            }
        }

        Ok(None)
    }

    pub fn cursor_up(&mut self) {
        if let Some(position) = self.state.selected() {
            if position > 0 {
                self.state.select(Some(position - 1))
            }
        }
    }

    pub fn cursor_down(&mut self, entries_len: usize) {
        if entries_len == 0 {
            return;
        }
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

    pub fn go_to_entry(&mut self, new_position: usize) {
        self.state.select(Some(new_position));
    }

    pub fn unselect_current(&mut self, state: &mut AppState) {
        let entry = self
            .state
            .selected()
            .and_then(|pos| state.selected.get(pos).cloned());

        if entry.is_none() {
            return;
        };

        let entry = entry.unwrap();

        state.remove(entry.relative_path);
    }

    pub fn unselect_all(&mut self, state: &mut AppState) {
        state.selected.clear()
    }
}

impl Component for ScrollList {
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
                return self.get_update(state);
            }
            Action::CursorDown => {
                self.cursor_down(state.selected.len());
                return self.get_update(state);
            }
            Action::CursorToTop => {
                self.go_to_top();
                return self.get_update(state);
            }
            Action::CursorToBottom => {
                self.go_to_bottom(state.selected.len());
                return self.get_update(state);
            }
            Action::ScriptFinished(entry, elapsed, crc) => {
                let new_position = state
                    .selected
                    .iter_mut()
                    .position(|s| s.relative_path == entry);

                if let Some(new_position) = new_position {
                    self.go_to_entry(new_position);
                }

                state
                    .selected
                    .iter_mut()
                    .filter(|s| s.relative_path == entry)
                    .for_each(|s| {
                        s.state = ScriptState::Finished;
                        s.elapsed = Some(elapsed);
                    });

                self.script_memory.insert(entry, crc, true)?;

                return self.get_update(state);
            }
            Action::ScriptError(entry, message, crc) => {
                let new_position = state
                    .selected
                    .iter_mut()
                    .position(|s| s.relative_path == entry);

                if let Some(new_position) = new_position {
                    self.go_to_entry(new_position);
                }

                state
                    .selected
                    .iter_mut()
                    .filter(|s| s.relative_path == entry)
                    .for_each(|s| {
                        s.state = ScriptState::Error;
                        s.error = Some(message.clone())
                    });

                if let Some(crc) = crc {
                    self.script_memory.insert(entry, crc, false)?;
                }

                return self.get_update(state);
            }
            Action::ScriptRunning(entry) => state
                .selected
                .iter_mut()
                .filter(|s| s.relative_path == entry)
                .for_each(|s| s.state = ScriptState::Running),
            Action::UnselectCurrent => {
                self.unselect_current(state);
                return Ok(None);
            }
            Action::UnselectAll => {
                self.unselect_all(state);
                return Ok(None);
            }
            Action::AddSelection(scripts) => {
                let mut only_new: Vec<Script> = scripts
                    .into_iter()
                    .filter(|s| !state.selected.iter().any(|r| r.relative_path == *s))
                    .map(|s| Script::none(&s))
                    .collect();
                state.selected.append(&mut only_new);
                state.selected.sort();

                self.update_selection(state);

                return self.get_update(state);
            }
            Action::RemoveSelection(scripts) => {
                state
                    .selected
                    .retain(|e| !scripts.contains(&e.relative_path));

                self.update_selection(state);

                return self.get_update(state);
            }
            Action::ScriptRun(skip_errors) => {
                let first_not_run_entry = state
                    .selected
                    .iter()
                    .find(|f| f.state == ScriptState::None)
                    .cloned();

                if first_not_run_entry.is_none() {
                    return Ok(None);
                }
                let entry = first_not_run_entry.unwrap();

                let full_path = self.base.join(&entry.relative_path);

                let connection = self.db.clone();
                let channel: Option<UnboundedSender<Action>> = self.command_tx.clone();
                let cloned = entry.clone();

                tokio::spawn(async move {
                    send_through_channel(
                        &channel,
                        Action::ScriptRunning(cloned.relative_path.clone()),
                    );

                    let now = Instant::now();
                    let content = tokio::fs::read_to_string(full_path).await;
                    match content {
                        Ok(content) => {
                            let result = connection.execute_script(&content).await;
                            let elapsed = now.elapsed().as_millis();
                            let hasher = Crc::<u32>::new(&CRC_32_ISO_HDLC);
                            let crc = hasher.checksum(content.as_bytes());
                            match result {
                                Ok(_) => {
                                    send_through_channel(
                                        &channel,
                                        Action::ScriptFinished(
                                            cloned.relative_path.clone(),
                                            elapsed,
                                            crc,
                                        ),
                                    );
                                    send_through_channel(
                                        &channel,
                                        Action::EntryStatusChanged(
                                            cloned.relative_path,
                                            crate::entries::EntryStatus::Finished(true),
                                        ),
                                    );
                                    send_through_channel(&channel, Action::ScriptRun(skip_errors));
                                }
                                Err(err) => {
                                    send_through_channel(
                                        &channel,
                                        Action::ScriptError(
                                            cloned.relative_path.clone(),
                                            err.to_string(),
                                            Some(crc),
                                        ),
                                    );
                                    send_through_channel(
                                        &channel,
                                        Action::EntryStatusChanged(
                                            cloned.relative_path,
                                            crate::entries::EntryStatus::Finished(false),
                                        ),
                                    );
                                    if skip_errors {
                                        send_through_channel(
                                            &channel,
                                            Action::ScriptRun(skip_errors),
                                        );
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            send_through_channel(
                                &channel,
                                Action::ScriptError(cloned.relative_path, err.to_string(), None),
                            );
                        }
                    }
                });

                //}
                return self.get_update(state);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &AppState) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(2),
                Constraint::Fill(1), // first row
            ])
            .split(area);

        let items: Vec<ListItem> = state
            .selected
            .iter()
            .map(|e| {
                let text = &e.relative_path;

                let style = match e.state {
                    ScriptState::None => Style::new().fg(Color::White),
                    ScriptState::Running => Style::new().fg(Color::Yellow),
                    ScriptState::Finished => Style::new().fg(Color::Green),
                    ScriptState::Error => Style::new().fg(Color::Red),
                };

                ListItem::new(Span::styled(text, style))
            })
            .collect();

        let list_draw = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title("Selected files"),
            )
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true);

        f.render_stateful_widget(&list_draw, rects[0], &mut self.state);

        Ok(())
    }
}
