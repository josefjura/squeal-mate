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
}

impl FilesystemRepository {
    /// Create a new filesystem repository
    pub fn new(root: PathBuf) -> InfraResult<Self> {
        let inner = Repository::new(root).map_err(|e| match e {
            crate::repository::RepositoryError::DoesNotExist => {
                InfraError::RepositoryNotFound("Repository path does not exist".to_string())
            }
            crate::repository::RepositoryError::NotUTF8 => InfraError::InvalidUtf8Path,
            crate::repository::RepositoryError::IOError(e) => {
                InfraError::IoError(std::io::Error::other(e))
            }
        })?;

        Ok(Self { inner })
    }
}

#[async_trait]
impl MigrationRepository for FilesystemRepository {
    async fn list_scripts(&self, directory: &Path) -> DomainResult<Vec<ScriptPath>> {
        let paths = self
            .inner
            .list_sql_files_in_directory(directory)
            .map_err(|e| InfraError::IoError(std::io::Error::other(e.to_string())))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn list_scripts_returns_sql_files_and_skips_hidden() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        std::fs::write(root.join("001_init.sql"), "SELECT 1;").unwrap();
        std::fs::write(root.join("readme.txt"), "not sql").unwrap();
        std::fs::write(root.join(".hidden.sql"), "SELECT 2;").unwrap();
        std::fs::write(root.join("_ignored.sql"), "SELECT 3;").unwrap();
        std::fs::create_dir(root.join("subdir")).unwrap();

        let repo = FilesystemRepository::new(root.to_path_buf()).unwrap();

        let mut scripts: Vec<String> = repo
            .list_scripts(Path::new(""))
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.to_string())
            .collect();
        scripts.sort();

        assert_eq!(scripts, vec!["001_init.sql".to_string()]);
    }

    #[tokio::test]
    async fn list_scripts_propagates_read_dir_error_instead_of_empty_list() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemRepository::new(temp_dir.path().to_path_buf()).unwrap();

        let result = repo.list_scripts(Path::new("does-not-exist")).await;

        assert!(result.is_err());
    }
}
