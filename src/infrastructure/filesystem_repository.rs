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
    root_path: PathBuf,  // Store owned path for returning references
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
                InfraError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        })?;

        Ok(Self { inner, root_path })
    }

    /// Get the underlying repository (for compatibility with existing code)
    pub fn inner(&self) -> &Repository {
        &self.inner
    }

    /// Get mutable access to the underlying repository
    pub fn inner_mut(&mut self) -> &mut Repository {
        &mut self.inner
    }
}

#[async_trait]
impl MigrationRepository for FilesystemRepository {
    async fn list_scripts(&self, _directory: &Path) -> DomainResult<Vec<ScriptPath>> {
        // Use current directory from inner repository
        let paths = self
            .inner
            .read_files_in_directory()
            .map_err(|e| InfraError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let script_paths: Result<Vec<_>, _> = paths
            .into_iter()
            .map(|p| ScriptPath::new(p))
            .collect();

        script_paths.map_err(Into::into)
    }

    async fn read_script(&self, path: &ScriptPath) -> DomainResult<MigrationScript> {
        let full_path = self.inner.base_as_path_buf().join(path.as_path());

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| InfraError::IoError(e))?;

        let script = MigrationScript::new(path.clone(), content);
        script.validate()?;

        Ok(script)
    }

    async fn get_children(&self, directory_path: &Path) -> DomainResult<Vec<ScriptPath>> {
        let paths = self.inner.get_children(directory_path.to_string_lossy().to_string());

        let script_paths: Result<Vec<_>, _> = paths
            .into_iter()
            .map(|p| ScriptPath::new(p))
            .collect();

        script_paths.map_err(Into::into)
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

        let script_paths: Result<Vec<_>, _> = paths
            .into_iter()
            .map(|p| ScriptPath::new(p))
            .collect();

        script_paths.map_err(Into::into)
    }

    fn enter_directory(&mut self, name: &str) -> bool {
        self.inner.open_directory(name);
        true // Existing implementation doesn't validate, always succeeds
    }

    fn leave_directory(&mut self) -> Option<String> {
        self.inner.leave_directory()
    }

    fn current_directory(&self) -> &Path {
        // Return the root path (we track navigation in the inner repository)
        &self.root_path
    }

    fn root_directory(&self) -> &Path {
        &self.root_path
    }
}
