use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List},
    Frame,
};

use crate::app::{App, Message, Screen};

/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui-org/ratatui/tree/master/examples
    match &app.current_screen {
        Screen::FileChooser { entries } => {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(1),
                ])
                .split(frame.size());

            let list_draw = List::new(entries)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double),
                )
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">>")
                .repeat_highlight_symbol(true);

            frame.render_widget(
                Span::styled(
                    app.base_path.to_str().unwrap_or(""),
                    Style::default().fg(Color::White),
                ),
                layout[0],
            );
            frame.render_stateful_widget(list_draw, layout[1], &mut app.ui_state.list);

            let message = &app.message;

            let (message_style, message) = match message {
                None => (Style::default().fg(Color::White), "".to_string()),
                Some(Message::Info(text)) => (Style::default().fg(Color::White), text.clone()),
                Some(Message::Success(text)) => (Style::default().fg(Color::Green), text.clone()),
                Some(Message::Error(text)) => (Style::default().fg(Color::Red), text.clone()),
            };

            frame.render_widget(
                Line::from(vec![
                    Span::styled("AEQ-CAC > ", Style::default().fg(Color::Red)),
                    Span::styled(message, message_style),
                ]),
                layout[2],
            );
        }
    }
}
