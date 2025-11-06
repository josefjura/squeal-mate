//! E2E tests that exercise the action system
//!
//! These tests demonstrate how to test App behavior by:
//! 1. Creating an App with mocked dependencies
//! 2. Initializing components
//! 3. Sending actions
//! 4. Verifying state changes

mod support;

use ratatui::layout::Size;
use squealmate::{
    action::Action,
    app::{App, AppState},
    script_memory::ScriptDatabase,
    ui::list::List,
};
use std::path::PathBuf;
use std::sync::Arc;
use support::{AppBuilder, MockFileSystem};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_app_initialization_with_actions() {
    // Setup: Create app with mocked filesystem
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let mut app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .build()
        .expect("Should build app");

    // Create action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();

    // Initialize components (this is what App::run does)
    let terminal_size = Size {
        width: 80,
        height: 24,
    };
    app.initialize_components(action_tx.clone(), terminal_size)
        .expect("Should initialize components");

    // Components may emit actions during initialization, drain them
    while action_rx.try_recv().is_ok() {}

    // Send an action
    action_tx
        .send(Action::Tick)
        .expect("Should send action");

    // Receive the action
    let received_action = action_rx.try_recv();
    assert!(received_action.is_ok(), "Should receive action");
    assert!(
        matches!(received_action.unwrap(), Action::Tick),
        "Should be a Tick action"
    );
}

#[tokio::test]
async fn test_action_channel_communication() {
    // This test demonstrates that actions can flow through the system
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&["001_init.sql"]);

    let mut app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .build()
        .expect("Should build app");

    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let terminal_size = Size {
        width: 100,
        height: 30,
    };

    app.initialize_components(action_tx.clone(), terminal_size)
        .expect("Should initialize");

    // Components may emit initialization actions, drain them
    while action_rx.try_recv().is_ok() {}

    // Send multiple actions
    action_tx.send(Action::Tick).unwrap();
    action_tx.send(Action::CursorUp).unwrap();
    action_tx.send(Action::CursorDown).unwrap();

    // Verify we can receive them
    let mut actions_received = vec![];
    while let Ok(action) = action_rx.try_recv() {
        actions_received.push(action);
    }

    assert_eq!(actions_received.len(), 3, "Should receive 3 actions");
}

#[tokio::test]
async fn test_app_state_manipulation() {
    // Test that we can manipulate app state directly
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
    ]);

    let mut app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .build()
        .expect("Should build app");

    // Initially empty
    assert_eq!(app.state.selected.len(), 0, "Should have no selected scripts");

    // Add a script
    app.state.add("001_init.sql".to_string());
    assert_eq!(app.state.selected.len(), 1, "Should have 1 selected script");

    // Toggle (should remove)
    app.state.toggle("001_init.sql".to_string());
    assert_eq!(app.state.selected.len(), 0, "Should have no selected scripts after toggle");

    // Toggle again (should add)
    app.state.toggle("001_init.sql".to_string());
    assert_eq!(app.state.selected.len(), 1, "Should have 1 selected script after re-toggle");
}

#[tokio::test]
async fn test_multiple_script_selection() {
    // Test selecting multiple scripts
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
        "004_orders.sql",
    ]);

    let mut app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .build()
        .expect("Should build app");

    // Select multiple scripts
    app.state.add("001_init.sql".to_string());
    app.state.add("002_users.sql".to_string());
    app.state.add("003_products.sql".to_string());

    assert_eq!(app.state.selected.len(), 3, "Should have 3 selected scripts");

    // Remove one
    app.state.remove("002_users.sql".to_string());
    assert_eq!(app.state.selected.len(), 2, "Should have 2 selected scripts");

    // Remove many
    app.state.remove_many(&["001_init.sql".to_string(), "003_products.sql".to_string()]);
    assert_eq!(app.state.selected.len(), 0, "Should have no selected scripts");
}
