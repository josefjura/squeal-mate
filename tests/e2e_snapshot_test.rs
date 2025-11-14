//! E2E snapshot tests using ratatui's TestBackend
//!
//! These tests render components to a test buffer and capture the output
//! for snapshot comparison using the `insta` crate.

mod support;

use ratatui::{backend::TestBackend, Terminal};
use squealmate::{
    action::Action,
    app::AppState,
    script_memory::{ScriptDatabase, ScriptResult},
    ui::{list::List, Component},
};
use std::path::PathBuf;
use std::sync::Arc;
use support::MockFileSystem;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_list_renders_empty_state() {
    // Setup: Empty filesystem
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone());

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // Setup TestBackend
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Setup action channel
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    list.register_action_handler(action_tx.clone()).unwrap();

    // Initialize list
    let terminal_size = ratatui::layout::Size {
        width: 80,
        height: 24,
    };
    list.init(terminal_size).unwrap();

    // Render
    let app_state = AppState::new();
    terminal
        .draw(|f| {
            list.draw(f, f.area(), &app_state).unwrap();
        })
        .unwrap();

    // Capture output as text lines
    let buffer = terminal.backend().buffer();
    let lines = extract_lines(buffer);

    // Snapshot the rendered lines
    insta::assert_debug_snapshot!("list_empty_state", lines);
}

#[tokio::test]
async fn test_list_renders_with_files() {
    // Setup: Filesystem with files
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // Setup TestBackend
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Setup action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    list.register_action_handler(action_tx.clone()).unwrap();

    // Initialize
    let terminal_size = ratatui::layout::Size {
        width: 80,
        height: 24,
    };
    list.init(terminal_size).unwrap();

    // Process actions until entries are loaded and statuses calculated
    let mut timeout = 50; // 50 iterations max
    let mut entries_loaded = false;
    let mut statuses_calculated = false;

    while timeout > 0 && (!entries_loaded || !statuses_calculated) {
        // Give async tasks time to run
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Process any pending actions
        while let Ok(action) = action_rx.try_recv() {
            let mut app_state = AppState::new();
            let _ = list.update(&mut app_state, action.clone());

            match action {
                Action::EntriesLoaded(_) => entries_loaded = true,
                Action::CalculateEntryStatus => {}
                Action::EntryStatusChanged(_, _) => statuses_calculated = true,
                _ => {}
            }
        }

        timeout -= 1;
    }

    // Render
    let app_state = AppState::new();
    terminal
        .draw(|f| {
            list.draw(f, f.area(), &app_state).unwrap();
        })
        .unwrap();

    // Capture output
    let buffer = terminal.backend().buffer();
    let lines = extract_lines(buffer);

    // Snapshot the rendered lines
    insta::assert_debug_snapshot!("list_with_files", lines);
}

fn extract_lines(buffer: &ratatui::buffer::Buffer) -> Vec<String> {
    let height = buffer.area().height as usize;
    let width = buffer.area().width as usize;
    let mut lines = Vec::new();

    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            let cell = buffer.cell((x as u16, y as u16)).unwrap();
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }

    lines
}

#[tokio::test]
async fn test_list_renders_with_execution_history() {
    // Setup: Files with execution history
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();

    // Pre-populate with execution results
    db.insert("001_init.sql".to_string(), 12345, ScriptResult::Success)
        .unwrap();
    db.insert("002_users.sql".to_string(), 67890, ScriptResult::Error)
        .unwrap();
    // 003_products.sql is never run

    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // Setup TestBackend
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Setup action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    list.register_action_handler(action_tx.clone()).unwrap();

    // Initialize
    let terminal_size = ratatui::layout::Size {
        width: 80,
        height: 24,
    };
    list.init(terminal_size).unwrap();

    // Process actions until entries are loaded and statuses calculated
    let mut timeout = 50;
    let mut entries_loaded = false;
    let mut statuses_calculated = false;

    while timeout > 0 && (!entries_loaded || !statuses_calculated) {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        while let Ok(action) = action_rx.try_recv() {
            let mut app_state = AppState::new();
            let _ = list.update(&mut app_state, action.clone());

            match action {
                Action::EntriesLoaded(_) => entries_loaded = true,
                Action::CalculateEntryStatus => {}
                Action::EntryStatusChanged(_, _) => statuses_calculated = true,
                _ => {}
            }
        }

        timeout -= 1;
    }

    // Render
    let app_state = AppState::new();
    terminal
        .draw(|f| {
            list.draw(f, f.area(), &app_state).unwrap();
        })
        .unwrap();

    // Capture output
    let buffer = terminal.backend().buffer();
    let lines = extract_lines(buffer);

    // Snapshot with execution status indicators
    insta::assert_debug_snapshot!("list_with_execution_history", lines);
}

#[tokio::test]
async fn test_list_renders_nested_directories() {
    // Setup: Nested directory structure
    // NOTE: This test verifies that the list component properly loads root-level entries
    // including both files and subdirectories. Directories start collapsed, so only
    // immediate children of the root are visible.
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "migrations/002_users.sql",
        "migrations/003_products.sql",
        "migrations/2024/004_orders.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // Setup TestBackend
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // Setup action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    list.register_action_handler(action_tx.clone()).unwrap();

    // Initialize
    let terminal_size = ratatui::layout::Size {
        width: 100,
        height: 30,
    };
    list.init(terminal_size).unwrap();

    // Process actions until entries are loaded and statuses calculated
    let mut timeout = 50;
    let mut entries_loaded = false;
    let mut statuses_calculated = false;

    while timeout > 0 && (!entries_loaded || !statuses_calculated) {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        while let Ok(action) = action_rx.try_recv() {
            let mut app_state = AppState::new();
            let _ = list.update(&mut app_state, action.clone());

            match action {
                Action::EntriesLoaded(_) => entries_loaded = true,
                Action::CalculateEntryStatus => {}
                Action::EntryStatusChanged(_, _) => statuses_calculated = true,
                _ => {}
            }
        }

        timeout -= 1;
    }

    // Render (directories start collapsed)
    let app_state = AppState::new();
    terminal
        .draw(|f| {
            list.draw(f, f.area(), &app_state).unwrap();
        })
        .unwrap();

    // Capture output
    let buffer = terminal.backend().buffer();
    let lines = extract_lines(buffer);

    // Snapshot showing root with collapsed subdirectories
    insta::assert_debug_snapshot!("list_nested_directories", lines);
}
