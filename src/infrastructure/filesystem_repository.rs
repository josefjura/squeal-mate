//! Filesystem-based implementation of MigrationRepository
//!
//! This wraps the existing Repository and implements the domain trait.

use crate::domain::{DomainResult, MigrationRepository, MigrationScript, ScriptPath};
use crate::infrastructure::error::{InfraError, InfraResult};
use crate::repository::Repository;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Filesystem repository implementation
pub struct FilesystemRepository {
    inner: Repository,
    root_path: PathBuf, // Store owned path for returning references
}

impl FilesystemRepository {
    /// Create a new filesystem repository
    pub fn new(root: PathBuf) -> InfraResult<Self> {
        let root_path = root.clone();
        let inner = Repository::new(root).map_err(|e| match e {
            crate::repository::RepositoryError::DoesNotExist => {
                InfraError::RepositoryNotFound("Repository path does not exist".to_string())
            }
            crate::repository::RepositoryError::NotUTF8 => InfraError::InvalidUtf8Path,
            crate::repository::RepositoryError::IOError(e) => {
                InfraError::IoError(std::io::Error::other(e))
            }
        })?;

        Ok(Self { inner, root_path })
    }
}

#[async_trait]
impl MigrationRepository for FilesystemRepository {
    async fn list_scripts(&self, directory: &Path) -> DomainResult<Vec<ScriptPath>> {
        // Read SQL files from the specified directory
        let mut paths = Vec::new();

        if let Ok(entries) = std::fs::read_dir(directory) {
            for entry in entries.flatten() {
                if let Ok(path) = entry.path().canonicalize() {
                    // Only include .sql files
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sql") {
                        // Skip hidden files
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if !name.starts_with('.') && !name.starts_with('_') {
                                paths.push(path);
                            }
                        }
                    }
                }
            }
        }

        let script_paths: Result<Vec<_>, _> = paths.into_iter().map(ScriptPath::new).collect();

        script_paths
    }

    async fn read_script(&self, path: &ScriptPath) -> DomainResult<MigrationScript> {
        let full_path = self.inner.base_as_path_buf().join(path.as_path());

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(InfraError::IoError)?;

        let script = MigrationScript::new(path.clone(), content);
        script.validate()?;

        Ok(script)
    }

    async fn get_children(&self, directory_path: &Path) -> DomainResult<Vec<ScriptPath>> {
        let paths = self
            .inner
            .get_children(directory_path.to_string_lossy().to_string());

        let script_paths: Result<Vec<_>, _> = paths.into_iter().map(ScriptPath::new).collect();

        script_paths
    }

    async fn get_scripts_after(
        &self,
        _directory: &Path,
        after: &ScriptPath,
    ) -> DomainResult<Vec<ScriptPath>> {
        let filename = after.filename().ok_or_else(|| {
            InfraError::ConfigError("Invalid script path - no filename".to_string())
        })?;

        let paths = self.inner.read_files_after(filename);

        let script_paths: Result<Vec<_>, _> = paths.into_iter().map(ScriptPath::new).collect();

        script_paths
    }

    async fn get_scripts_after_in_current(
        &self,
        after_name: &str,
    ) -> DomainResult<Vec<ScriptPath>> {
        let paths = self
            .inner
            .read_files_after_in_directory(after_name)
            .map_err(|e| InfraError::IoError(std::io::Error::other(e.to_string())))?;

        let script_paths: Result<Vec<_>, _> = paths.into_iter().map(ScriptPath::new).collect();

        script_paths
    }

    async fn get_scripts_in_current(&self) -> DomainResult<Vec<ScriptPath>> {
        let paths = self
            .inner
            .read_files_in_directory()
            .map_err(|e| InfraError::IoError(std::io::Error::other(e.to_string())))?;

        let script_paths: Result<Vec<_>, _> = paths.into_iter().map(ScriptPath::new).collect();

        script_paths
    }

    async fn get_scripts_after_global(&self, after_name: &str) -> DomainResult<Vec<ScriptPath>> {
        let paths = self.inner.read_files_after(after_name);

        let script_paths: Result<Vec<_>, _> = paths.into_iter().map(ScriptPath::new).collect();

        script_paths
    }
}
