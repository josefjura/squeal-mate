use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders},
    layout::Size,
};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::{Action, PanelFocus},
    app::AppState,
    infrastructure::Settings,
    tui::Frame,
};

/// The main unified view component that replaces the dual-screen approach
/// This provides a lazygit-style interface with:
/// - File tree browser (left panel, 60% width)
/// - Script details/preview (right top, 40% width, 60% height)
/// - Execution log (right bottom, 40% width, 40% height)
/// - Persistent command bar (bottom)
pub struct UnifiedView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Settings,

    // Sub-components
    file_tree: Box<dyn Component + Send + Sync>,
    script_preview: Box<dyn Component + Send + Sync>,
    execution_log: Box<dyn Component + Send + Sync>,
    command_bar: Box<dyn Component + Send + Sync>,

    // Panel focus state
    focused_panel: PanelFocus,
}

impl UnifiedView {
    pub fn new(
        file_tree: Box<dyn Component + Send + Sync>,
        script_preview: Box<dyn Component + Send + Sync>,
        execution_log: Box<dyn Component + Send + Sync>,
        command_bar: Box<dyn Component + Send + Sync>,
    ) -> Self {
        Self {
            command_tx: None,
            config: Settings::default(),
            file_tree,
            script_preview,
            execution_log,
            command_bar,
            focused_panel: PanelFocus::FileTree,
        }
    }

    fn layout(&self, area: Rect) -> (Rect, Rect, Rect, Rect) {
        // Main vertical split: content (top) + command bar (bottom)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),        // Content area (flexible)
                Constraint::Length(3),      // Command bar (fixed height)
            ])
            .split(area);

        let content_area = main_chunks[0];
        let command_bar_area = main_chunks[1];

        // Horizontal split: left panel (60%) + right panel (40%)
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60),  // File tree
                Constraint::Percentage(40),  // Right side (details + log)
            ])
            .split(content_area);

        let file_tree_area = horizontal_chunks[0];
        let right_panel = horizontal_chunks[1];

        // Vertical split of right panel: details (60%) + log (40%)
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60),  // Script preview/details
                Constraint::Percentage(40),  // Execution log
            ])
            .split(right_panel);

        let preview_area = right_chunks[0];
        let log_area = right_chunks[1];

        (file_tree_area, preview_area, log_area, command_bar_area)
    }
}

impl Component for UnifiedView {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx.clone());

        // Register with all sub-components
        self.file_tree.register_action_handler(tx.clone())?;
        self.script_preview.register_action_handler(tx.clone())?;
        self.execution_log.register_action_handler(tx.clone())?;
        self.command_bar.register_action_handler(tx)?;

        Ok(())
    }

    fn register_config_handler(&mut self, config: Settings) -> Result<()> {
        self.config = config.clone();

        // Register with all sub-components
        self.file_tree.register_config_handler(config.clone())?;
        self.script_preview.register_config_handler(config.clone())?;
        self.execution_log.register_config_handler(config.clone())?;
        self.command_bar.register_config_handler(config)?;

        Ok(())
    }

    fn init(&mut self, area: Size) -> Result<()> {
        // Convert Size to Rect for layout calculation
        let rect = Rect::new(0, 0, area.width, area.height);
        let (file_area, preview_area, log_area, cmd_area) = self.layout(rect);

        // Init sub-components with their sizes
        self.file_tree.init(Size::new(file_area.width, file_area.height))?;
        self.script_preview.init(Size::new(preview_area.width, preview_area.height))?;
        self.execution_log.init(Size::new(log_area.width, log_area.height))?;
        self.command_bar.init(Size::new(cmd_area.width, cmd_area.height))?;

        Ok(())
    }

    fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        // Route events based on focused panel
        match self.focused_panel {
            PanelFocus::FileTree => self.file_tree.handle_events(event),
            PanelFocus::ScriptPreview => self.script_preview.handle_events(event),
            PanelFocus::ExecutionLog => self.execution_log.handle_events(event),
        }
    }

    fn update(&mut self, state: &mut AppState, action: Action) -> Result<Option<Action>> {
        // Handle panel switching actions
        match action {
            Action::FocusNextPanel => {
                self.focused_panel = match self.focused_panel {
                    PanelFocus::FileTree => PanelFocus::ScriptPreview,
                    PanelFocus::ScriptPreview => PanelFocus::ExecutionLog,
                    PanelFocus::ExecutionLog => PanelFocus::FileTree,
                };
                // Notify other components of focus change
                if let Some(ref tx) = self.command_tx {
                    let _ = tx.send(Action::PanelFocusChanged(self.focused_panel));
                }
                return Ok(Some(Action::Render));
            }
            Action::FocusPreviousPanel => {
                self.focused_panel = match self.focused_panel {
                    PanelFocus::FileTree => PanelFocus::ExecutionLog,
                    PanelFocus::ExecutionLog => PanelFocus::ScriptPreview,
                    PanelFocus::ScriptPreview => PanelFocus::FileTree,
                };
                // Notify other components of focus change
                if let Some(ref tx) = self.command_tx {
                    let _ = tx.send(Action::PanelFocusChanged(self.focused_panel));
                }
                return Ok(Some(Action::Render));
            }
            _ => {}
        }

        // Update all components (they filter actions they care about)
        let mut result_action = None;

        if let Some(action) = self.file_tree.update(state, action.clone())? {
            result_action = Some(action);
        }
        if let Some(action) = self.script_preview.update(state, action.clone())? {
            result_action = Some(action);
        }
        if let Some(action) = self.execution_log.update(state, action.clone())? {
            result_action = Some(action);
        }
        if let Some(action) = self.command_bar.update(state, action)? {
            result_action = Some(action);
        }

        Ok(result_action)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, state: &AppState) -> Result<()> {
        let (file_area, preview_area, log_area, cmd_area) = self.layout(area);

        // Draw file tree with focus indicator
        let file_block = Block::default()
            .borders(Borders::ALL)
            .title(" Files ")
            .border_style(if matches!(self.focused_panel, PanelFocus::FileTree) {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let file_inner = file_block.inner(file_area);
        f.render_widget(file_block, file_area);
        self.file_tree.draw(f, file_inner, state)?;

        // Draw script preview with focus indicator
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .title(" Script Details ")
            .border_style(if matches!(self.focused_panel, PanelFocus::ScriptPreview) {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let preview_inner = preview_block.inner(preview_area);
        f.render_widget(preview_block, preview_area);
        self.script_preview.draw(f, preview_inner, state)?;

        // Draw execution log with focus indicator
        let log_block = Block::default()
            .borders(Borders::ALL)
            .title(" Execution Log ")
            .border_style(if matches!(self.focused_panel, PanelFocus::ExecutionLog) {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let log_inner = log_block.inner(log_area);
        f.render_widget(log_block, log_area);
        self.execution_log.draw(f, log_inner, state)?;

        // Draw command bar (no border, always visible)
        self.command_bar.draw(f, cmd_area, state)?;

        Ok(())
    }
}
