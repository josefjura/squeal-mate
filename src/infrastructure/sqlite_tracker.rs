//! SQLite-based execution tracker

use crate::domain::{Checksum, DomainResult, ExecutionResult, ExecutionTracker, ScriptPath, ScriptStatus};
use crate::infrastructure::error::InfraError;
use crate::script_memory::ScriptDatabase;
use async_trait::async_trait;

/// SQLite implementation of execution tracker
pub struct SqliteTracker {
    db: ScriptDatabase,
}

impl SqliteTracker {
    pub async fn new() -> Result<Self, InfraError> {
        let db = ScriptDatabase::new().await.map_err(|e| {
            InfraError::SqliteError(rusqlite::Error::InvalidQuery) // TODO: Better error conversion
        })?;

        Ok(Self { db })
    }
}

#[async_trait]
impl ExecutionTracker for SqliteTracker {
    async fn record_execution(
        &self,
        path: &ScriptPath,
        result: &ExecutionResult,
    ) -> DomainResult<()> {
        self.db
            .insert(
                path.to_string(),
                result.checksum.value(),
                result.success,
            )
            .map_err(|e| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?; // TODO: Better error

        Ok(())
    }

    async fn get_status(
        &self,
        path: &ScriptPath,
        current_checksum: Checksum,
    ) -> DomainResult<ScriptStatus> {
        // Use the business logic method to determine status
        let has_been_executed = self.has_been_executed(path).await?;
        let stored_checksum = self.get_last_checksum(path).await?;

        // Get the old status to check if last execution succeeded
        let old_status = self
            .db
            .get_file_status(&path.to_string(), &current_checksum.value())
            .map_err(|_e| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?;

        let last_execution_succeeded = matches!(
            old_status,
            crate::entries::EntryStatus::Finished(true)
        );

        // Use domain business logic to determine status
        let domain_status = ScriptStatus::from_execution_history(
            has_been_executed,
            last_execution_succeeded,
            current_checksum,
            stored_checksum,
        );

        Ok(domain_status)
    }

    async fn has_been_executed(&self, path: &ScriptPath) -> DomainResult<bool> {
        // Simple check - if we can get a checksum, it's been executed
        let checksum = self.get_last_checksum(path).await?;
        Ok(checksum.is_some())
    }

    async fn get_last_checksum(&self, path: &ScriptPath) -> DomainResult<Option<Checksum>> {
        // For now, we'll use a dummy checksum to query status
        // The real implementation would query the database directly
        let dummy = Checksum::from_value(0);
        let status = self.db
            .get_file_status(&path.to_string(), &dummy.value())
            .map_err(|e| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?;

        match status {
            crate::entries::EntryStatus::NeverStarted | crate::entries::EntryStatus::Unknown => {
                Ok(None)
            }
            _ => Ok(Some(dummy)), // TODO: Get actual checksum from database
        }
    }
}
