use std::{collections::HashMap, path::PathBuf};

use color_eyre::eyre::{Ok, Result};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, List, ListState},
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, tui::Frame};

pub struct ScrollList {
    command_tx: Option<UnboundedSender<Action>>,
    config: HashMap<String, String>,
    state: ListState,
    entries: Vec<PathBuf>,
}

impl ScrollList {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            config: HashMap::<String, String>::default(),
            state: ListState::default().with_selected(Some(0)),
            entries: vec![],
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
            _ => {}
        }
        Ok(None)
    }

    fn update_background(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::SelectScripts(scripts) => {
                self.entries.clear();
                self.entries.extend(scripts);
                return Ok(None);
            }
            Action::AppendScripts(mut scripts) => {
                self.entries.append(&mut scripts);
                return Ok(None);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let list_draw = List::new(self.entries.iter().filter_map(|f| f.as_path().to_str()))
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
