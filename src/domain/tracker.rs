//! Execution tracking trait

use crate::domain::error::DomainResult;
use crate::domain::script::{Checksum, ScriptPath};
use crate::domain::script_status::{ExecutionResult, ScriptStatus};
use async_trait::async_trait;

/// Tracks script execution history
#[async_trait]
pub trait ExecutionTracker: Send + Sync {
    /// Record the result of a script execution
    async fn record_execution(
        &self,
        path: &ScriptPath,
        result: &ExecutionResult,
    ) -> DomainResult<()>;

    /// Get the execution status for a script
    async fn get_status(
        &self,
        path: &ScriptPath,
        current_checksum: Checksum,
    ) -> DomainResult<ScriptStatus>;

    /// Get database-only status (without checksum comparison)
    /// This returns the status based on execution history only, without checking for modifications
    async fn get_database_status(&self, path: &ScriptPath) -> DomainResult<ScriptStatus>;
}
