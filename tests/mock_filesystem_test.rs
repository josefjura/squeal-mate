//! Tests for MockFileSystem

mod support;

use squealmate::infrastructure::FileSystem;
use std::path::PathBuf;
use support::MockFileSystem;

#[tokio::test]
async fn test_mock_filesystem_basic() {
    let root = PathBuf::from("/test");
    let fs = MockFileSystem::new(root.clone()).with_files(&["001_init.sql", "002_users.sql"]);

    // List root directory
    let entries = fs.list_directory(&root).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "001_init.sql");
    assert!(!entries[0].is_directory);

    // List SQL files recursively
    let files = fs.list_sql_files_recursive(&root).await.unwrap();
    assert_eq!(files.len(), 2);
}

#[tokio::test]
async fn test_mock_filesystem_with_subdirs() {
    let root = PathBuf::from("/test");
    let fs = MockFileSystem::new(root.clone()).with_files(&[
        "migrations/001_init.sql",
        "migrations/002_users.sql",
        "migrations/2024/003_products.sql",
    ]);

    // List all SQL files
    let all_files = fs.list_sql_files_recursive(&root).await.unwrap();
    assert_eq!(all_files.len(), 3);

    // List only files in migrations/2024
    let subdir_files = fs
        .list_sql_files_recursive(&root.join("migrations/2024"))
        .await
        .unwrap();
    assert_eq!(subdir_files.len(), 1);
}
