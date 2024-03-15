use crate::app::{App, AppResult, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles the key events and updates the state of [`App`].
pub async fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match &mut app.current_screen {
        Screen::FileChooser { entries } => match key_event.code {
            // Exit application on `ESC` or `q`
            KeyCode::Char('q') => {
                app.quit();
            }
            // Exit application on `Ctrl-C`
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if key_event.modifiers == KeyModifiers::CONTROL {
                    app.quit();
                }
            }
            // Counter handlers
            KeyCode::Up => {
                app.cursor_up();
            }
            KeyCode::Down => {
                let entries_len = entries.len();
                app.cursor_down(entries_len);
            }
            KeyCode::Home => {
                app.go_to_top();
            }
            KeyCode::End => {
                let entries_len = entries.len();
                app.go_to_bottom(entries_len);
            }
            KeyCode::Enter => {
                app.open_selected_directory();
            }
            KeyCode::Backspace | KeyCode::Esc => {
                app.leave_directory();
            }
            KeyCode::Char('s') => app.execute_selected_script().await,
            // Other handlers you could add here.
            _ => {}
        },
    }

    Ok(())
}
