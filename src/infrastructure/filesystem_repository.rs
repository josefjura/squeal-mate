//! Filesystem-based implementation of MigrationRepository
//!
//! This wraps the existing Repository and implements the domain trait.

use crate::domain::{DomainResult, MigrationRepository, MigrationScript, ScriptPath};
use crate::infrastructure::error::{InfraError, InfraResult};
use crate::repository::Repository;
use async_trait::async_trait;
use std::path::PathBuf;

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
    async fn read_script(&self, path: &ScriptPath) -> DomainResult<MigrationScript> {
        let full_path = self.inner.base_as_path_buf().join(path.as_path());

        let content = tokio::fs::read_to_string(&full_path)
            .await
            .map_err(InfraError::IoError)?;

        let script = MigrationScript::new(path.clone(), content);
        script.validate()?;

        Ok(script)
    }
}
