//! Repository traits for domain layer
//!
//! These traits define the interface for accessing migration scripts
//! without coupling to infrastructure details.

use crate::domain::error::DomainResult;
use crate::domain::script::{MigrationScript, ScriptPath};
use async_trait::async_trait;
use std::path::Path;

/// Repository for accessing migration scripts from storage
#[async_trait]
#[allow(dead_code)] // Some methods reserved for future features
pub trait MigrationRepository: Send + Sync {
    /// List all script paths in a directory
    async fn list_scripts(&self, directory: &Path) -> DomainResult<Vec<ScriptPath>>;

    /// Read a script's content
    async fn read_script(&self, path: &ScriptPath) -> DomainResult<MigrationScript>;

    /// Get all child scripts under a directory path (recursive)
    async fn get_children(&self, directory_path: &Path) -> DomainResult<Vec<ScriptPath>>;

    /// Get scripts after a given script in the same directory
    async fn get_scripts_after(
        &self,
        directory: &Path,
        after: &ScriptPath,
    ) -> DomainResult<Vec<ScriptPath>>;

    /// Get scripts after a given script name (in current directory, non-recursive)
    async fn get_scripts_after_in_current(
        &self,
        after_name: &str,
    ) -> DomainResult<Vec<ScriptPath>>;

    /// Get all scripts in the current directory (non-recursive)
    async fn get_scripts_in_current(&self) -> DomainResult<Vec<ScriptPath>>;

    /// Get all scripts starting from a specific path (recursive, globally from repo root)
    async fn get_scripts_after_global(
        &self,
        after_name: &str,
    ) -> DomainResult<Vec<ScriptPath>>;
}
