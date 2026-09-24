//! Migration service - orchestrates script execution and status tracking

use crate::action::Action;
use crate::domain::{
    DomainError, DomainResult, ExecutionTracker, MigrationScript, ScriptExecutor, ScriptPath,
    ScriptStatus,
};
use crate::infrastructure::FileExplorer;
use std::sync::Arc;
use tokio::sync::mpsc::{error::SendError, UnboundedSender};

impl From<SendError<Action>> for DomainError {
    fn from(_: SendError<Action>) -> Self {
        DomainError::ActionChannelClosed
    }
}

/// Service for managing migration operations
pub struct MigrationService {
    explorer: Arc<FileExplorer>,
    executor: Arc<dyn ScriptExecutor>,
    tracker: Arc<dyn ExecutionTracker>,
}

impl MigrationService {
    /// Create a new migration service
    pub fn new(
        explorer: Arc<FileExplorer>,
        executor: Arc<dyn ScriptExecutor>,
        tracker: Arc<dyn ExecutionTracker>,
    ) -> Self {
        Self {
            explorer,
            executor,
            tracker,
        }
    }

    /// Execute a single migration script
    pub async fn execute_script(
        &self,
        script: &MigrationScript,
        tx: &UnboundedSender<Action>,
    ) -> DomainResult<()> {
        // Notify that execution is starting
        tx.send(Action::ScriptRunning(script.path.to_string()))?;

        // Execute the script
        let result = self.executor.execute(script).await?;

        // Record the execution
        self.tracker.record_execution(&script.path, &result).await?;

        // Get the updated status after recording
        let updated_status = self
            .tracker
            .get_status(&script.path, result.checksum)
            .await?;

        // Convert ScriptStatus to EntryStatus for UI updates (using From trait)
        let entry_status = crate::entries::EntryStatus::from(updated_status);

        // Update the entry status in the UI
        tx.send(Action::EntryStatusChanged(
            script.path.to_string(),
            entry_status,
        ))?;

        // Notify completion (for execution log)
        if result.success {
            tx.send(Action::ScriptFinished(
                script.path.to_string(),
                result.elapsed_ms,
                result.checksum.value(),
            ))?;
        } else {
            tx.send(Action::ScriptError(
                script.path.to_string(),
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
                Some(result.checksum.value()),
            ))?;
        }

        Ok(())
    }

    /// Get database status for all scripts and dispatch status updates
    /// Does NOT check for file modifications - use check_for_changes() for that
    pub fn calculate_statuses(&self, scripts: Vec<ScriptPath>, tx: &UnboundedSender<Action>) {
        let tracker = self.tracker.clone();
        let tx = tx.clone();
        let total = scripts.len();

        tokio::spawn(async move {
            for (index, script_path) in scripts.iter().enumerate() {
                // Send progress update
                if let Err(e) = tx.send(Action::StatusCalculationProgress(index + 1, total)) {
                    log::error!("Action channel closed: {}", e);
                    return;
                }

                // Get status from database only (no CRC checking)
                match tracker.get_database_status(script_path).await {
                    Ok(status) => {
                        // Convert ScriptStatus to EntryStatus (using From trait)
                        let entry_status = crate::entries::EntryStatus::from(status);

                        if let Err(e) = tx.send(Action::EntryStatusChanged(
                            script_path.to_string(),
                            entry_status,
                        )) {
                            log::error!("Action channel closed: {}", e);
                            return;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get status for {}: {}", script_path, e);
                    }
                }
            }
        });
    }

    /// Check for file modifications by comparing CRC checksums
    /// Only checks scripts that have been executed before (not NeverRun or Skipped)
    pub fn check_for_changes(&self, scripts: Vec<ScriptPath>, tx: &UnboundedSender<Action>) {
        let tracker = self.tracker.clone();
        let explorer = self.explorer.clone();
        let tx = tx.clone();
        let total = scripts.len();

        tokio::spawn(async move {
            for (index, script_path) in scripts.iter().enumerate() {
                // Send progress update
                if let Err(e) = tx.send(Action::StatusCalculationProgress(index + 1, total)) {
                    log::error!("Action channel closed: {}", e);
                    return;
                }

                // Read the script to get its current checksum
                match explorer.read_script(script_path).await {
                    Ok(script) => {
                        match tracker.get_status(script_path, script.checksum).await {
                            Ok(status) => {
                                // Only update if status is Modified
                                if status == ScriptStatus::Modified {
                                    if let Err(e) = tx.send(Action::EntryStatusChanged(
                                        script_path.to_string(),
                                        crate::entries::EntryStatus::Changed,
                                    )) {
                                        log::error!("Action channel closed: {}", e);
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to check CRC for {}: {}", script_path, e);
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

    /// Whether a script is currently marked to be skipped
    pub async fn is_skipped(&self, path: &ScriptPath) -> DomainResult<bool> {
        self.tracker.is_skipped(path).await
    }

    /// Mark a script as skipped
    pub async fn mark_skipped(&self, path: &ScriptPath) -> DomainResult<()> {
        self.tracker.mark_skipped(path).await
    }

    /// Remove skip status from a script
    pub async fn unmark_skipped(&self, path: &ScriptPath) -> DomainResult<()> {
        self.tracker.unmark_skipped(path).await
    }

    /// Get the relative paths of all scripts that have execution history
    pub async fn get_all_executed_scripts(
        &self,
    ) -> DomainResult<std::collections::HashSet<String>> {
        self.tracker.get_all_executed_scripts().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fakes::{FakeExecutionTracker, FakeScriptExecutor};
    use crate::domain::Checksum;
    use crate::entries::EntryStatus;

    /// A temp directory holding real script files, read through a real `FileExplorer`
    struct TestRepository {
        dir: tempfile::TempDir,
    }

    impl TestRepository {
        fn new() -> Self {
            Self {
                dir: tempfile::tempdir().unwrap(),
            }
        }

        fn add_script(&self, script: MigrationScript) {
            std::fs::write(self.dir.path().join(script.path.as_path()), &script.content).unwrap();
        }

        fn explorer(&self) -> Arc<FileExplorer> {
            Arc::new(FileExplorer::new(self.dir.path().to_path_buf()).unwrap())
        }
    }

    fn script(path: &str, content: &str) -> MigrationScript {
        MigrationScript::new(ScriptPath::new(path).unwrap(), content.to_string())
    }

    fn service(
        repository: &TestRepository,
        executor: FakeScriptExecutor,
        tracker: FakeExecutionTracker,
    ) -> MigrationService {
        MigrationService::new(repository.explorer(), Arc::new(executor), Arc::new(tracker))
    }

    fn channel() -> (
        UnboundedSender<Action>,
        tokio::sync::mpsc::UnboundedReceiver<Action>,
    ) {
        tokio::sync::mpsc::unbounded_channel()
    }

    #[tokio::test]
    async fn execute_script_success_dispatches_running_then_finished() {
        let executor = FakeScriptExecutor::new();
        let tracker = FakeExecutionTracker::new();
        let svc = service(&TestRepository::new(), executor, tracker);
        let (tx, mut rx) = channel();
        let script = script("migration.sql", "SELECT 1;");

        svc.execute_script(&script, &tx).await.unwrap();

        assert!(matches!(
            rx.recv().await,
            Some(Action::ScriptRunning(p)) if p == "migration.sql"
        ));
        assert!(matches!(
            rx.recv().await,
            Some(Action::EntryStatusChanged(p, EntryStatus::NeverStarted)) if p == "migration.sql"
        ));
        assert!(matches!(
            rx.recv().await,
            Some(Action::ScriptFinished(p, _, checksum))
                if p == "migration.sql" && checksum == script.checksum.value()
        ));
    }

    #[tokio::test]
    async fn execute_script_records_result_with_tracker() {
        let tracker = Arc::new(FakeExecutionTracker::new());
        let svc = MigrationService::new(
            TestRepository::new().explorer(),
            Arc::new(FakeScriptExecutor::new()),
            tracker.clone(),
        );
        let (tx, _rx) = channel();
        let script = script("migration.sql", "SELECT 1;");

        svc.execute_script(&script, &tx).await.unwrap();

        let recorded = tracker.recorded_execution(&script.path);
        assert!(recorded.is_some());
        assert!(recorded.unwrap().success);
    }

    #[tokio::test]
    async fn execute_script_failure_dispatches_script_error() {
        let executor = FakeScriptExecutor::new();
        let checksum = Checksum::from_content("SELECT 1;");
        executor.set_result(crate::domain::ExecutionResult::failure(
            "syntax error".to_string(),
            5,
            checksum,
        ));
        let tracker = FakeExecutionTracker::new();
        let svc = service(&TestRepository::new(), executor, tracker);
        let (tx, mut rx) = channel();
        let script = script("migration.sql", "SELECT 1;");

        svc.execute_script(&script, &tx).await.unwrap();

        rx.recv().await; // ScriptRunning
        rx.recv().await; // EntryStatusChanged
        let finished = rx.recv().await;
        assert!(matches!(
            finished,
            Some(Action::ScriptError(p, msg, Some(c)))
                if p == "migration.sql" && msg == "syntax error" && c == checksum.value()
        ));
    }

    #[tokio::test]
    async fn calculate_statuses_dispatches_status_for_each_script() {
        let tracker = FakeExecutionTracker::new();
        let path_a = ScriptPath::new("a.sql").unwrap();
        let path_b = ScriptPath::new("b.sql").unwrap();
        tracker.set_status(&path_a, ScriptStatus::UpToDate);
        tracker.set_status(&path_b, ScriptStatus::NeverRun);
        let svc = service(&TestRepository::new(), FakeScriptExecutor::new(), tracker);
        let (tx, mut rx) = channel();

        svc.calculate_statuses(vec![path_a, path_b], &tx);

        let mut statuses = Vec::new();
        for _ in 0..4 {
            match rx.recv().await.unwrap() {
                Action::EntryStatusChanged(path, status) => statuses.push((path, status)),
                Action::StatusCalculationProgress(_, _) => {}
                other => panic!("unexpected action: {other:?}"),
            }
        }

        assert_eq!(
            statuses,
            vec![
                ("a.sql".to_string(), EntryStatus::Finished(true)),
                ("b.sql".to_string(), EntryStatus::NeverStarted),
            ]
        );
    }

    #[tokio::test]
    async fn check_for_changes_only_flags_modified_scripts() {
        let repository = TestRepository::new();
        let unchanged = script("unchanged.sql", "SELECT 1;");
        let changed = script("changed.sql", "SELECT 2;");
        repository.add_script(unchanged.clone());
        repository.add_script(changed.clone());

        let tracker = FakeExecutionTracker::new();
        // Tracker holds a stale checksum for "changed.sql", matching for "unchanged.sql".
        tracker.set_status(&unchanged.path, ScriptStatus::UpToDate);
        tracker.set_status(&changed.path, ScriptStatus::Modified);

        let svc = service(&repository, FakeScriptExecutor::new(), tracker);
        let (tx, mut rx) = channel();

        svc.check_for_changes(vec![unchanged.path.clone(), changed.path.clone()], &tx);

        let mut changed_paths = Vec::new();
        for _ in 0..3 {
            match rx.recv().await.unwrap() {
                Action::EntryStatusChanged(path, EntryStatus::Changed) => changed_paths.push(path),
                Action::EntryStatusChanged(path, other) => {
                    panic!("unexpected status {other:?} for {path}")
                }
                Action::StatusCalculationProgress(_, _) => {}
                other => panic!("unexpected action: {other:?}"),
            }
        }

        assert_eq!(changed_paths, vec!["changed.sql".to_string()]);
    }

    #[tokio::test]
    async fn skip_lifecycle_round_trips_through_tracker() {
        let tracker = FakeExecutionTracker::new();
        let svc = service(&TestRepository::new(), FakeScriptExecutor::new(), tracker);
        let path = ScriptPath::new("migration.sql").unwrap();

        assert!(!svc.is_skipped(&path).await.unwrap());

        svc.mark_skipped(&path).await.unwrap();
        assert!(svc.is_skipped(&path).await.unwrap());

        svc.unmark_skipped(&path).await.unwrap();
        assert!(!svc.is_skipped(&path).await.unwrap());
    }

    #[tokio::test]
    async fn test_connection_delegates_to_executor() {
        let executor = FakeScriptExecutor::new();
        executor.set_connection_result(Err(crate::domain::DomainError::ExecutionFailed(
            "unreachable".to_string(),
        )));
        let svc = service(
            &TestRepository::new(),
            executor,
            FakeExecutionTracker::new(),
        );

        let result = svc.test_connection().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_script_reports_closed_action_channel() {
        let svc = service(
            &TestRepository::new(),
            FakeScriptExecutor::new(),
            FakeExecutionTracker::new(),
        );
        let (tx, rx) = channel();
        drop(rx);

        let result = svc.execute_script(&script("a.sql", "SELECT 1"), &tx).await;

        assert!(matches!(result, Err(DomainError::ActionChannelClosed)));
    }
}
