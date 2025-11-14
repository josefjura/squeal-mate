//! E2E tests for App using AppBuilder and mocks

mod support;

use std::path::PathBuf;
use std::sync::Arc;
use support::{AppBuilder, MockFileSystem};

#[test]
fn test_app_initializes_with_mock_filesystem() {
    // Setup: Create mock filesystem with test scripts
    let root = PathBuf::from("/test");
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_users.sql",
        "migrations/003_products.sql",
    ]);

    // Build app with mocked dependencies
    let app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .build()
        .expect("App should build successfully");

    // Verify app structure
    assert_eq!(app.screens.len(), 3, "App should have 3 screens");
}

#[test]
fn test_app_builds_with_custom_database() {
    // Create a temporary directory for the app
    let temp_dir = std::env::temp_dir().join("squealmate_test_db");
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create a test database
    let test_db =
        squealmate::script_memory::ScriptDatabase::new_test().expect("Should create test database");

    // Build app with custom database
    let app = AppBuilder::new_test()
        .with_root(temp_dir.clone())
        .with_database(test_db)
        .build();

    assert!(app.is_ok(), "App should build with custom database");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_app_with_full_mock_dependencies() {
    // This test demonstrates the complete testing infrastructure:
    // - Mock filesystem (no real files)
    // - Test database (isolated temp database)
    // - AppBuilder (dependency injection)

    let root = PathBuf::from("/mock/migrations");

    // Setup mock filesystem with a realistic structure
    let mock_fs = MockFileSystem::new(root.clone()).with_files(&[
        "001_init.sql",
        "002_create_users_table.sql",
        "migrations/003_add_products.sql",
        "migrations/2024/004_orders.sql",
    ]);

    // Setup test database
    let test_db =
        squealmate::script_memory::ScriptDatabase::new_test().expect("Should create test database");

    // Build app with all mocked dependencies
    let app = AppBuilder::new_test()
        .with_root(root)
        .with_filesystem(Arc::new(mock_fs))
        .with_database(test_db)
        .build()
        .expect("App should build with all mocked dependencies");

    // Verify the app structure
    assert_eq!(
        app.screens.len(),
        3,
        "Should have 3 screens: Unified, FileChooser, ScriptRunner"
    );

    // Verify we have the right screen modes
    let modes: Vec<_> = app.screens.iter().map(|s| s.mode).collect();
    assert!(modes.contains(&squealmate::screen::Mode::Unified));
    assert!(modes.contains(&squealmate::screen::Mode::FileChooser));
    assert!(modes.contains(&squealmate::screen::Mode::ScriptRunner));
}
