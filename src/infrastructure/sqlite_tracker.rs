//! SQLite-based execution tracker

use crate::domain::{
    Checksum, DomainResult, ExecutionResult, ExecutionTracker, ScriptPath, ScriptStatus,
};
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
            .insert(path.to_string(), result.checksum.value(), script_result)
            .map_err(|_| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?; // TODO: Better error

        Ok(())
    }

    async fn get_status(
        &self,
        path: &ScriptPath,
        current_checksum: Checksum,
    ) -> DomainResult<ScriptStatus> {
        // Get the stored record (if exists)
        let record = self
            .db
            .get_script_record(&path.to_string())
            .map_err(|_e| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?;

        // Check if the script is marked as skipped
        if let Some(ref rec) = record {
            if rec.result == ScriptResult::Skipped {
                // Script is marked as skipped
                return Ok(ScriptStatus::Skipped);
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

    async fn get_database_status(&self, path: &ScriptPath) -> DomainResult<ScriptStatus> {
        // Get the stored record (if exists)
        let record = self
            .db
            .get_script_record(&path.to_string())
            .map_err(|_e| InfraError::SqliteError(rusqlite::Error::InvalidQuery))?;

        // Check if the script is marked as skipped
        if let Some(ref rec) = record {
            if rec.result == ScriptResult::Skipped {
                return Ok(ScriptStatus::Skipped);
            }
        }

        // Determine status from database record only (no checksum comparison)
        match record {
            Some(rec) => {
                match rec.result {
                    ScriptResult::Success => Ok(ScriptStatus::UpToDate), // Assume up-to-date without CRC check
                    ScriptResult::Error => Ok(ScriptStatus::Failed {
                        error: "Previous execution failed".to_string(),
                    }),
                    ScriptResult::Skipped => Ok(ScriptStatus::Skipped),
                }
            }
            None => Ok(ScriptStatus::NeverRun),
        }
    }
}
