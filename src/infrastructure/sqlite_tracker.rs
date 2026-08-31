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
        let db = ScriptDatabase::new().await?;

        Ok(Self { db })
    }

    #[cfg(test)]
    fn new_test() -> Self {
        Self {
            db: ScriptDatabase::new_test().expect("failed to create test database"),
        }
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
            .map_err(InfraError::from)?;

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
            .map_err(InfraError::from)?;

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
            .map_err(InfraError::from)?;

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

    async fn is_skipped(&self, path: &ScriptPath) -> DomainResult<bool> {
        Ok(self.db.is_skipped(&path.to_string()))
    }

    async fn mark_skipped(&self, path: &ScriptPath) -> DomainResult<()> {
        self.db
            .mark_skipped(path.to_string())
            .map_err(InfraError::from)?;
        Ok(())
    }

    async fn unmark_skipped(&self, path: &ScriptPath) -> DomainResult<()> {
        self.db
            .unmark_skipped(path.to_string())
            .map_err(InfraError::from)?;
        Ok(())
    }

    async fn get_all_executed_scripts(&self) -> DomainResult<std::collections::HashSet<String>> {
        let scripts = self
            .db
            .get_all_executed_scripts()
            .map_err(InfraError::from)?;
        Ok(scripts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mark_skipped_makes_is_skipped_true() {
        let tracker = SqliteTracker::new_test();
        let path = ScriptPath::new("foo.sql").unwrap();

        assert!(!tracker.is_skipped(&path).await.unwrap());

        tracker.mark_skipped(&path).await.unwrap();

        assert!(tracker.is_skipped(&path).await.unwrap());
    }

    #[tokio::test]
    async fn unmark_skipped_makes_is_skipped_false() {
        let tracker = SqliteTracker::new_test();
        let path = ScriptPath::new("foo.sql").unwrap();

        tracker.mark_skipped(&path).await.unwrap();
        assert!(tracker.is_skipped(&path).await.unwrap());

        tracker.unmark_skipped(&path).await.unwrap();

        assert!(!tracker.is_skipped(&path).await.unwrap());
    }

    #[tokio::test]
    async fn mark_skipped_reflected_in_get_status_and_get_database_status() {
        let tracker = SqliteTracker::new_test();
        let path = ScriptPath::new("foo.sql").unwrap();

        tracker.mark_skipped(&path).await.unwrap();

        assert_eq!(
            tracker
                .get_status(&path, Checksum::from_value(123))
                .await
                .unwrap(),
            ScriptStatus::Skipped
        );
        assert_eq!(
            tracker.get_database_status(&path).await.unwrap(),
            ScriptStatus::Skipped
        );
    }

    #[tokio::test]
    async fn get_all_executed_scripts_includes_skipped_and_recorded_scripts() {
        let tracker = SqliteTracker::new_test();
        let skipped = ScriptPath::new("skipped.sql").unwrap();
        let run = ScriptPath::new("run.sql").unwrap();

        tracker.mark_skipped(&skipped).await.unwrap();
        tracker
            .record_execution(&run, &ExecutionResult::success(10, Checksum::from_value(1)))
            .await
            .unwrap();

        let executed = tracker.get_all_executed_scripts().await.unwrap();

        assert!(executed.contains("skipped.sql"));
        assert!(executed.contains("run.sql"));
    }
}
