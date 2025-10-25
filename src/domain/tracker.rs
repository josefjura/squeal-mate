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

    /// Check if a script has ever been executed
    async fn has_been_executed(&self, path: &ScriptPath) -> DomainResult<bool>;

    /// Get the last stored checksum for a script
    async fn get_last_checksum(&self, path: &ScriptPath) -> DomainResult<Option<Checksum>>;
}
