//! Script execution traits

use crate::domain::error::DomainResult;
use crate::domain::script::MigrationScript;
use crate::domain::script_status::ExecutionResult;
use async_trait::async_trait;

/// Executes migration scripts against a database
#[async_trait]
pub trait ScriptExecutor: Send + Sync {
    /// Execute a migration script
    async fn execute(&self, script: &MigrationScript) -> DomainResult<ExecutionResult>;

    /// Test the database connection
    async fn test_connection(&self) -> DomainResult<()>;
}
