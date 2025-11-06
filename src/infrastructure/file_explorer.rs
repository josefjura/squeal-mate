//! Simple file system explorer for UI file browsing
//!
//! This provides a lightweight interface for browsing SQL files and directories,
//! without the heavyweight domain abstractions needed for script execution.

use color_eyre::eyre::Result;
use std::path::{Path, PathBuf};

/// Entry in a directory listing - simple struct for UI display
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
}

/// Simple file system explorer for browsing SQL migrations
#[derive(Debug, Clone)]
pub struct FileExplorer {
    root: PathBuf,
}

impl FileExplorer {
    /// Create a new file explorer rooted at the given directory
    pub fn new(root: PathBuf) -> Result<Self> {
        if !root.exists() {
            return Err(color_eyre::eyre::eyre!("Path does not exist: {}", root.display()));
        }
        if !root.is_dir() {
            return Err(color_eyre::eyre::eyre!("Path is not a directory: {}", root.display()));
        }
        Ok(Self { root })
    }

    /// List all entries (files and directories) in a directory
    pub async fn list_directory(&self, dir: &Path) -> Result<Vec<Entry>> {
        let dir = dir.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let mut entries = Vec::new();

            let read_dir = std::fs::read_dir(&dir)
                .map_err(|e| color_eyre::eyre::eyre!("Failed to read directory {}: {}", dir.display(), e))?;

            for entry_result in read_dir {
                let entry = entry_result?;
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files/directories (starting with . or _)
                if name.starts_with('.') || name.starts_with('_') {
                    continue;
                }

                let metadata = entry.metadata()?;

                if metadata.is_dir() {
                    entries.push(Entry {
                        name,
                        path,
                        is_directory: true,
                    });
                } else if metadata.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sql") {
                    // Only include .sql files
                    entries.push(Entry {
                        name,
                        path,
                        is_directory: false,
                    });
                }
            }

            // Sort: directories first, then by name
            entries.sort_by(|a, b| {
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });

            Ok(entries)
        })
        .await?
    }

    /// List only SQL files in a directory (no directories, non-recursive)
    #[allow(dead_code)]
    pub async fn list_sql_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let dir = dir.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let mut files = Vec::new();

            let read_dir = std::fs::read_dir(&dir)
                .map_err(|e| color_eyre::eyre::eyre!("Failed to read directory {}: {}", dir.display(), e))?;

            for entry_result in read_dir {
                let entry = entry_result?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "sql" {
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");

                            // Skip hidden files
                            if !name.starts_with('.') && !name.starts_with('_') {
                                files.push(path);
                            }
                        }
                    }
                }
            }

            // Sort by name
            files.sort_by(|a, b| {
                a.file_name().cmp(&b.file_name())
            });

            Ok(files)
        })
        .await?
    }

    /// List all SQL files recursively in a directory tree
    pub async fn list_sql_files_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let dir = dir.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let mut files = Vec::new();
            Self::collect_sql_files(&dir, &mut files)?;

            // Sort by full path
            files.sort();

            Ok(files)
        })
        .await?
    }

    /// Helper function to recursively collect SQL files
    fn collect_sql_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        let read_dir = std::fs::read_dir(dir)
            .map_err(|e| {
                color_eyre::eyre::eyre!("Failed to read directory {}: {}", dir.display(), e)
            })?;

        for entry_result in read_dir {
            let entry = entry_result?;
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            // Skip hidden files/directories
            if name.starts_with('.') || name.starts_with('_') {
                continue;
            }

            if path.is_dir() {
                // Recurse into subdirectory
                Self::collect_sql_files(&path, files)?;
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "sql" {
                        files.push(path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Read the contents of a SQL file
    #[allow(dead_code)]
    pub async fn read_file(&self, path: &Path) -> Result<String> {
        tokio::fs::read_to_string(path).await
            .map_err(|e| color_eyre::eyre::eyre!("Failed to read file {}: {}", path.display(), e))
    }

    /// Get the root directory
    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_list_directory_with_sql_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create some test files
        fs::write(root.join("001_test.sql"), "SELECT 1;").unwrap();
        fs::write(root.join("002_test.sql"), "SELECT 2;").unwrap();
        fs::write(root.join("readme.txt"), "Not a SQL file").unwrap();
        fs::write(root.join("_hidden.sql"), "Hidden").unwrap();
        fs::create_dir(root.join("subdir")).unwrap();

        let explorer = FileExplorer::new(root.to_path_buf()).unwrap();
        let entries = explorer.list_directory(root).await.unwrap();

        // Should have: subdir (directory), 001_test.sql, 002_test.sql
        // Should NOT have: readme.txt, _hidden.sql
        assert_eq!(entries.len(), 3);

        // First should be directory
        assert!(entries[0].is_directory);
        assert_eq!(entries[0].name, "subdir");

        // Then SQL files in order
        assert!(!entries[1].is_directory);
        assert_eq!(entries[1].name, "001_test.sql");

        assert!(!entries[2].is_directory);
        assert_eq!(entries[2].name, "002_test.sql");
    }

    #[tokio::test]
    async fn test_list_sql_files_only() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("001_test.sql"), "SELECT 1;").unwrap();
        fs::write(root.join("002_test.sql"), "SELECT 2;").unwrap();
        fs::create_dir(root.join("subdir")).unwrap();

        let explorer = FileExplorer::new(root.to_path_buf()).unwrap();
        let files = explorer.list_sql_files(root).await.unwrap();

        // Should only have SQL files, not directories
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("001_test.sql"));
        assert!(files[1].ends_with("002_test.sql"));
    }

    #[tokio::test]
    async fn test_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let file_path = root.join("test.sql");

        let content = "SELECT * FROM users;";
        fs::write(&file_path, content).unwrap();

        let explorer = FileExplorer::new(root.to_path_buf()).unwrap();
        let read_content = explorer.read_file(&file_path).await.unwrap();

        assert_eq!(read_content, content);
    }

    #[test]
    fn test_new_with_nonexistent_path() {
        let result = FileExplorer::new(PathBuf::from("/this/does/not/exist"));
        assert!(result.is_err());
    }
}
