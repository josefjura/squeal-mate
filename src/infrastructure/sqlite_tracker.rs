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
        let status = self
            .db
            .get_file_status(&path.to_string(), &current_checksum.value())
            .map_err(|e| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?;

        // Convert old EntryStatus to new ScriptStatus
        let domain_status = match status {
            crate::entries::EntryStatus::NeverStarted => ScriptStatus::NeverRun,
            crate::entries::EntryStatus::Finished(true) => ScriptStatus::UpToDate,
            crate::entries::EntryStatus::Finished(false) => ScriptStatus::Failed {
                error: "Previous execution failed".to_string(),
            },
            crate::entries::EntryStatus::Changed => ScriptStatus::Modified,
            crate::entries::EntryStatus::Unknown => ScriptStatus::NeverRun,
            crate::entries::EntryStatus::Directory => ScriptStatus::NeverRun, // Shouldn't happen
        };

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
