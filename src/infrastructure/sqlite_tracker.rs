//! SQLite-based execution tracker

use crate::domain::{Checksum, DomainResult, ExecutionResult, ExecutionTracker, ScriptPath, ScriptStatus};
use crate::infrastructure::error::InfraError;
use crate::script_memory::{ScriptDatabase, ScriptResult};
use async_trait::async_trait;

/// SQLite implementation of execution tracker
pub struct SqliteTracker {
    db: ScriptDatabase,
}

impl SqliteTracker {
    pub async fn new() -> Result<Self, InfraError> {
        let db = ScriptDatabase::new()
            .await
            .map_err(|_| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?; // TODO: Better error conversion

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
        let script_result = if result.success {
            ScriptResult::Success
        } else {
            ScriptResult::Error
        };

        self.db
            .insert(
                path.to_string(),
                result.checksum.value(),
                script_result,
            )
            .map_err(|_| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?; // TODO: Better error

        Ok(())
    }

    async fn get_status(
        &self,
        path: &ScriptPath,
        current_checksum: Checksum,
    ) -> DomainResult<ScriptStatus> {
        // Get the stored record (if exists)
        let record = self.db
            .get_script_record(&path.to_string())
            .map_err(|_e| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?;

        // Check if the script is marked as skipped
        if let Some(ref rec) = record {
            if rec.result == ScriptResult::Skipped {
                // Script is marked as skipped, return NeverRun
                // The UI layer will convert this to EntryStatus::Skipped based on the database record
                return Ok(ScriptStatus::NeverRun);
            }
        }

        // Determine execution history from the stored record
        let (has_been_executed, last_execution_succeeded, stored_checksum) = match record {
            Some(rec) => {
                let success = rec.result == ScriptResult::Success;
                (true, success, Some(Checksum::from_value(rec.crc)))
            }
            None => (false, false, None),
        };

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
        // Query the database for the stored record
        let record = self.db
            .get_script_record(&path.to_string())
            .map_err(|_| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?;

        // Return the stored checksum if found
        Ok(record.map(|r| Checksum::from_value(r.crc)))
    }
}
