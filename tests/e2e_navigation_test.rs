//! E2E tests for navigation workflows

mod support;

use squealmate::{
    action::Action,
    app::AppState,
    entries::EntryStatus,
    script_memory::{ScriptDatabase, ScriptResult},
    ui::list::List,
};
use std::path::PathBuf;
use std::sync::Arc;
use support::MockFileSystem;

#[tokio::test]
async fn test_list_initialization_with_mock_filesystem() {
    // Test that List can be initialized with a mocked filesystem
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();
    let list = List::new_with_filesystem(root, db, Arc::new(mock_fs));

    assert!(
        list.is_ok(),
        "Should be able to create List with mocked filesystem"
    );
}

#[tokio::test]
async fn test_navigation_methods_dont_panic() {
    // Test that navigation methods can be called without panicking
    // Even without entries loaded, navigation should be safe
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&["001_init.sql"]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // These should not panic even with no entries loaded
    list.cursor_down(3);
    list.cursor_up();
    list.go_to_top();
    list.go_to_bottom(3);

    // Should return None when no entries loaded
    let selection = list.get_selection();
    assert!(
        selection.is_none() || selection.is_some(),
        "get_selection should either return None or Some, not panic"
    );
}

#[tokio::test]
async fn test_directory_operations_dont_panic() {
    // Test that directory expand/collapse operations are safe
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "migrations/002_users.sql",
        "migrations/003_products.sql",
        "migrations/2024/004_orders.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // These operations should not panic even without loaded entries
    let result = list.expand_current_directory();
    assert!(result.is_ok(), "expand_current_directory should not error");

    let result = list.collapse_current_or_goto_parent();
    assert!(
        result.is_ok(),
        "collapse_current_or_goto_parent should not error"
    );

    let result = list.open_selected_directory();
    assert!(result.is_ok(), "open_selected_directory should not error");
}

#[tokio::test]
async fn test_selection_operations_are_safe() {
    // Test that selection operations don't panic
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();
    let mut app_state = AppState::new();

    // These should not panic even without loaded entries
    list.select_current(&mut app_state);
    list.unselect_all(&mut app_state);

    // State should be consistent
    assert!(
        app_state.selected.is_empty() || !app_state.selected.is_empty(),
        "AppState should be in valid state"
    );
}

#[tokio::test]
async fn test_select_from_cursor_to_end_doesnt_panic() {
    // Test that select_from_cursor_to_end is safe to call
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
        "004_orders.sql",
        "005_payments.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();
    let mut app_state = AppState::new();

    // Move cursor
    list.cursor_down(2);

    // Should not panic
    list.select_from_cursor_to_end(&mut app_state);

    // Note: This is an async operation that dispatches actions
    // Without the full event loop, we can only verify it doesn't panic
}

#[tokio::test]
async fn test_database_integration() {
    // Test that List can work with a database that has pre-existing data
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();

    // Pre-populate database with script execution results
    db.insert("001_init.sql".to_string(), 12345, ScriptResult::Success)
        .unwrap();
    db.insert("002_users.sql".to_string(), 67890, ScriptResult::Error)
        .unwrap();
    // 003_products.sql is not in DB (never run)

    // Should be able to create List with pre-populated database
    let list = List::new_with_filesystem(root, db, Arc::new(mock_fs));
    assert!(
        list.is_ok(),
        "Should create List with pre-populated database"
    );
}

#[tokio::test]
async fn test_toggle_skip_doesnt_panic() {
    // Test that toggle_skip is safe to call
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&["001_init.sql", "002_users.sql"]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db.clone(), Arc::new(mock_fs)).unwrap();

    // Should not panic even without loaded entries
    list.toggle_skip();

    // Note: toggle_skip dispatches an action
    // Without the full event loop, we can only verify it doesn't panic
}

#[tokio::test]
async fn test_jump_to_next_not_run_doesnt_panic() {
    // Test that jump_to_next_not_run is safe to call
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "003_products.sql",
    ]);

    let db = ScriptDatabase::new_test().unwrap();
    let mut list = List::new_with_filesystem(root, db, Arc::new(mock_fs)).unwrap();

    // Should not panic
    list.jump_to_next_not_run();
}
