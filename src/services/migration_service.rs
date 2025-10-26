//! Migration service - orchestrates script execution and status tracking

use crate::action::Action;
use crate::domain::{
    Checksum, DomainResult, ExecutionTracker, MigrationRepository, MigrationScript, ScriptExecutor,
    ScriptPath, ScriptStatus,
};
use crate::services::ActionDispatcher;
use std::sync::Arc;
use tokio::time::Instant;

/// Service for managing migration operations
pub struct MigrationService {
    repository: Arc<dyn MigrationRepository>,
    executor: Arc<dyn ScriptExecutor>,
    tracker: Arc<dyn ExecutionTracker>,
}

impl MigrationService {
    /// Create a new migration service
    pub fn new(
        repository: Arc<dyn MigrationRepository>,
        executor: Arc<dyn ScriptExecutor>,
        tracker: Arc<dyn ExecutionTracker>,
    ) -> Self {
        Self {
            repository,
            executor,
            tracker,
        }
    }

    /// Execute a single migration script
    pub async fn execute_script(
        &self,
        script: &MigrationScript,
        dispatcher: &ActionDispatcher,
    ) -> DomainResult<()> {
        // Notify that execution is starting
        dispatcher.dispatch(Action::ScriptRunning(script.path.to_string()));

        // Execute the script
        let result = self.executor.execute(script).await?;

        // Record the execution
        self.tracker.record_execution(&script.path, &result).await?;

        // Notify completion
        if result.success {
            dispatcher.dispatch(Action::ScriptFinished(
                script.path.to_string(),
                result.elapsed_ms,
                result.checksum.value(),
            ));
        } else {
            dispatcher.dispatch(Action::ScriptError(
                script.path.to_string(),
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
                Some(result.checksum.value()),
            ));
        }

        Ok(())
    }

    /// Get the status of a script
    pub async fn get_script_status(
        &self,
        path: &ScriptPath,
        checksum: Checksum,
    ) -> DomainResult<ScriptStatus> {
        self.tracker.get_status(path, checksum).await
    }

    /// Calculate checksums for all scripts and dispatch status updates
    pub fn calculate_statuses(
        &self,
        scripts: Vec<ScriptPath>,
        dispatcher: &ActionDispatcher,
    ) {
        let tracker = self.tracker.clone();
        let repo = self.repository.clone();
        let disp = dispatcher.clone();
        let total = scripts.len();

        tokio::spawn(async move {
            for (index, script_path) in scripts.iter().enumerate() {
                // Send progress update
                disp.dispatch(Action::StatusCalculationProgress(index + 1, total));
                // Read the script to get its checksum
                match repo.read_script(&script_path).await {
                    Ok(script) => {
                        match tracker.get_status(&script_path, script.checksum).await {
                            Ok(status) => {
                                // Use business logic methods for logging/diagnostics
                                if status.needs_attention() {
                                    log::debug!("Script {} needs attention", script_path);
                                }
                                if !status.can_execute() {
                                    log::debug!("Script {} is up-to-date, skipping", script_path);
                                }

                                // Convert ScriptStatus to EntryStatus for backwards compatibility
                                let entry_status = match status {
                                    ScriptStatus::NeverRun => {
                                        crate::entries::EntryStatus::NeverStarted
                                    }
                                    ScriptStatus::UpToDate => {
                                        crate::entries::EntryStatus::Finished(true)
                                    }
                                    ScriptStatus::Modified => crate::entries::EntryStatus::Changed,
                                    ScriptStatus::Failed { .. } => {
                                        crate::entries::EntryStatus::Finished(false)
                                    }
                                    ScriptStatus::Running => crate::entries::EntryStatus::Unknown,
                                };

                                disp.dispatch(Action::EntryStatusChanged(
                                    script_path.to_string(),
                                    entry_status,
                                ));
                            }
                            Err(e) => {
                                log::error!("Failed to get status for {}: {}", script_path, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read script {}: {}", script_path, e);
                    }
                }
            }
        });
    }

    /// Test the database connection
    pub async fn test_connection(&self) -> DomainResult<()> {
        self.executor.test_connection().await
    }
}
