//! Mock filesystem implementation for testing

use async_trait::async_trait;
use color_eyre::eyre::Result;
use squealmate::infrastructure::{FileSystem, FsEntry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Mock filesystem for testing
///
/// Allows programmatic setup of directory structure without touching real FS.
/// All operations are in-memory and instant.
#[derive(Clone)]
pub struct MockFileSystem {
    root: PathBuf,
    /// Map of directory path -> list of entries in that directory
    directories: Arc<Mutex<HashMap<PathBuf, Vec<FsEntry>>>>,
    /// Set of all SQL file paths (for recursive listing)
    sql_files: Arc<Mutex<Vec<PathBuf>>>,
}

impl MockFileSystem {
    /// Create a new mock filesystem with the given root
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: root.clone(),
            directories: Arc::new(Mutex::new(HashMap::new())),
            sql_files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a SQL file to the mock filesystem
    ///
    /// Example: `mock_fs.add_file("migrations/001_init.sql")`
    pub fn add_file(&self, relative_path: &str) {
        let full_path = self.root.join(relative_path);
        let parent = full_path.parent().unwrap().to_path_buf(); // Clone parent path
        let name = full_path.file_name().unwrap().to_string_lossy().to_string();

        // Add to SQL files list
        {
            let mut files = self.sql_files.lock().unwrap();
            files.push(full_path.clone());
        }

        // Add entry to parent directory
        {
            let mut dirs = self.directories.lock().unwrap();
            let entries = dirs.entry(parent.clone()).or_insert_with(Vec::new);

            // Don't add duplicates
            if !entries.iter().any(|e| e.name == name) {
                entries.push(FsEntry {
                    name,
                    path: full_path,
                    is_directory: false,
                });
            }
        }

        // Also ensure all parent directories exist
        self.ensure_directory_exists(&parent);
    }

    /// Add a directory to the mock filesystem
    ///
    /// Example: `mock_fs.add_directory("migrations/2024")`
    pub fn add_directory(&self, relative_path: &str) {
        let full_path = self.root.join(relative_path);
        self.ensure_directory_exists(&full_path);
    }

    /// Ensure a directory and all its parents exist in the mock
    fn ensure_directory_exists(&self, dir: &Path) {
        let mut dirs = self.directories.lock().unwrap();

        // Create entry for this directory if it doesn't exist
        dirs.entry(dir.to_path_buf()).or_insert_with(Vec::new);

        // If this directory has a parent, add it as an entry
        if let Some(parent) = dir.parent() {
            if parent != self.root {
                let name = dir.file_name().unwrap().to_string_lossy().to_string();
                let entries = dirs.entry(parent.to_path_buf()).or_insert_with(Vec::new);

                // Don't add duplicates
                if !entries.iter().any(|e| e.name == name && e.is_directory) {
                    entries.push(FsEntry {
                        name,
                        path: dir.to_path_buf(),
                        is_directory: true,
                    });
                }

                // Recursively ensure parent exists
                drop(dirs); // Release lock before recursive call
                self.ensure_directory_exists(parent);
            }
        }
    }

    /// Builder-style method to add multiple files at once
    pub fn with_files(self, files: &[&str]) -> Self {
        for file in files {
            self.add_file(file);
        }
        self
    }

    /// Builder-style method to add multiple directories at once
    pub fn with_directories(self, dirs: &[&str]) -> Self {
        for dir in dirs {
            self.add_directory(dir);
        }
        self
    }
}

#[async_trait]
impl FileSystem for MockFileSystem {
    async fn list_directory(&self, dir: &Path) -> Result<Vec<FsEntry>> {
        let dirs = self.directories.lock().unwrap();

        Ok(dirs
            .get(dir)
            .cloned()
            .unwrap_or_default())
    }

    async fn list_sql_files_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let files = self.sql_files.lock().unwrap();

        // Filter files that are under the requested directory
        Ok(files
            .iter()
            .filter(|f| f.starts_with(dir))
            .cloned()
            .collect())
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_filesystem_basic() {
        let root = PathBuf::from("/test");
        let fs = MockFileSystem::new(root.clone())
            .with_files(&["001_init.sql", "002_users.sql"]);

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
        let fs = MockFileSystem::new(root.clone())
            .with_files(&[
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
}
