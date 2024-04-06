use async_trait::async_trait;
// use crossterm::style::{StyledContent, Stylize};
use std::{collections::HashMap, path::PathBuf};

use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};
use tokio::{sync::mpsc::UnboundedSender, time::Instant};

use super::Component;
use crate::{
    action::Action,
    db::Database,
    entries::{Entry, ResultLine, ResultState},
    tui::Frame,
};

pub struct ScrollList {
    command_tx: Option<UnboundedSender<Action>>,
    config: HashMap<String, String>,
    state: ListState,
    results: Vec<ResultLine>,
    db: Database,
    base: PathBuf,
}

impl ScrollList {
    pub fn new(db: Database, base: PathBuf) -> Self {
        Self {
            command_tx: None,
            config: HashMap::<String, String>::default(),
            state: ListState::default().with_selected(Some(0)),
            results: vec![],
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
}

#[async_trait]
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
                return self.get_update();
            }
            Action::CursorDown => {
                self.cursor_down(self.results.len());
                return self.get_update();
            }
            Action::CursorToTop => {
                self.go_to_top();
                return self.get_update();
            }
            Action::CursorToBottom => {
                self.go_to_bottom(self.results.len());
                return self.get_update();
            }
            Action::RemoveSelectedScript => {
                if let Some(pos) = self.state.selected() {
                    let entry = self.results.get(pos);
                    if let Some(entry) = entry {
                        return Ok(Some(Action::RemoveScript(entry.result.clone())));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    async fn update_background(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::ScriptFinished(entry, elapsed) => self
                .results
                .iter_mut()
                .filter(|s| s.result == entry)
                .for_each(|s| {
                    s.state = ResultState::FINISHED;
                    s.elapsed = Some(elapsed);
                }),
            Action::ScriptError(entry, message) => self
                .results
                .iter_mut()
                .filter(|s| s.result == entry)
                .for_each(|s| {
                    s.state = ResultState::ERROR;
                    s.error = Some(message.clone())
                }),
            Action::ScriptRunning(entry) => self
                .results
                .iter_mut()
                .filter(|s| s.result == entry)
                .for_each(|s| s.state = ResultState::RUNNING),
            Action::SelectScripts(scripts) => {
                self.results.clear();
                self.results
                    .extend(scripts.iter().map(|s| ResultLine::None(s)));
                self.results.sort();

                return self.get_update();
            }
            Action::AppendScripts(scripts) => {
                let mut only_new: Vec<ResultLine> = scripts
                    .into_iter()
                    .filter(|s| !self.results.iter().any(|r| r.result == *s))
                    .map(|s| ResultLine::None(&s))
                    .collect();
                self.results.append(&mut only_new);
                self.results.sort();
                return self.get_update();
            }
            Action::RemoveScript(entry) => {
                self.results.retain(|e| e.result != entry);
                return self.get_update();
            }
            Action::RemoveAllSelectedScripts => {
                self.results.clear();
                return self.get_update();
            }
            Action::ScriptRun => {
                let entry = self
                    .results
                    .iter()
                    .skip_while(|f| f.state != ResultState::NONE)
                    .cloned()
                    .next();

                if entry.is_none() {
                    return Ok(None);
                }
                let entry = entry.unwrap();
                if let Entry::File(_) = entry.result {
                    let rel_dir = entry.result.get_full_path()?;
                    let full_path = self.base.join(&rel_dir);

                    let connection = self.db.clone();
                    let channel: Option<UnboundedSender<Action>> = self.command_tx.clone();
                    let cloned = entry;

                    tokio::spawn(async move {
                        send_through_channel(
                            &channel,
                            Action::ScriptRunning(cloned.result.clone()),
                        );

                        let now = Instant::now();
                        let result = connection.execute_script(full_path).await;
                        let elapsed = now.elapsed().as_millis();

                        match result {
                            Ok(_) => {
                                send_through_channel(
                                    &channel,
                                    Action::ScriptFinished(cloned.result, elapsed),
                                );
                                send_through_channel(&channel, Action::ScriptRun);
                            }
                            Err(err) => {
                                send_through_channel(
                                    &channel,
                                    Action::ScriptError(cloned.result, err.to_string()),
                                );
                            }
                        }
                    });
                }
                //}
                return self.get_update();
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(2),
                Constraint::Fill(1), // first row
            ])
            .split(area);

        let items: Vec<ListItem> = self
            .results
            .iter()
            .filter_map(|e| {
                let text = e.result.get_full_path().ok()?.to_str().map(String::from);

                match e.state {
                    ResultState::NONE => {
                        text.map(|f| ListItem::new(f).style(Style::new().fg(Color::White)))
                    }
                    ResultState::RUNNING => {
                        text.map(|f| ListItem::new(f).style(Style::new().fg(Color::Yellow)))
                    }
                    ResultState::FINISHED => {
                        text.map(|f| ListItem::new(f).style(Style::new().fg(Color::Green)))
                    }
                    ResultState::ERROR => {
                        text.map(|f| ListItem::new(f).style(Style::new().fg(Color::Red)))
                    }
                }
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

impl ScrollList {
    fn get_update(&self) -> Result<Option<Action>> {
        if let Some(pos) = self.state.selected() {
            let entry = self.results.get(pos);
            if let Some(entry) = entry {
                return Ok(Some(Action::ScriptHighlighted(Some(entry.clone()))));
            } else {
                return Ok(Some(Action::ScriptHighlighted(None)));
            }
        }
        return Ok(None);
    }
}

fn send_through_channel(channel: &Option<UnboundedSender<Action>>, action: Action) {
    if let Some(channel) = channel {
        if let Err(error) = channel.send(action) {
            log::error!("{}", error);
        }
    }
}
