//! Filesystem abstraction for testability
//!
//! This module provides a trait-based abstraction over filesystem operations,
//! allowing for easy mocking in tests.

use async_trait::async_trait;
use color_eyre::eyre::Result;
use std::path::{Path, PathBuf};

/// Entry in a directory (file or subdirectory)
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
}

/// Abstraction over filesystem operations for testability
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// List all entries (files and directories) in a directory
    async fn list_directory(&self, dir: &Path) -> Result<Vec<Entry>>;

    /// List all SQL files recursively in a directory tree
    async fn list_sql_files_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>>;

    /// Get the root directory
    fn root(&self) -> &Path;
}
