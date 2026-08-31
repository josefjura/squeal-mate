//! Fake adapters for the domain traits, used to unit-test orchestration
//! logic (e.g. `MigrationService`) without touching a real filesystem,
//! SQLite database, or SQL Server connection.

use crate::domain::error::{DomainError, DomainResult};
use crate::domain::executor::ScriptExecutor;
use crate::domain::repository::MigrationRepository;
use crate::domain::script::{Checksum, MigrationScript, ScriptPath};
use crate::domain::script_status::{ExecutionResult, ScriptStatus};
use crate::domain::tracker::ExecutionTracker;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

/// In-memory fake for `MigrationRepository`.
///
/// Scripts are pre-registered with `with_script` / `add_script`; `read_script`
/// looks them up by path and errors with `ScriptNotFound` for anything else.
#[derive(Default)]
pub struct FakeMigrationRepository {
    scripts: Mutex<HashMap<ScriptPath, MigrationScript>>,
}

#[allow(dead_code)] // Some helpers reserved for tests not yet written
impl FakeMigrationRepository {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style helper to register a script before handing the fake to a service.
    pub fn with_script(self, script: MigrationScript) -> Self {
        self.add_script(script);
        self
    }

    pub fn add_script(&self, script: MigrationScript) {
        self.scripts
            .lock()
            .unwrap()
            .insert(script.path.clone(), script);
    }
}

#[async_trait]
impl MigrationRepository for FakeMigrationRepository {
    async fn list_scripts(&self, _directory: &Path) -> DomainResult<Vec<ScriptPath>> {
        Ok(self.scripts.lock().unwrap().keys().cloned().collect())
    }

    async fn read_script(&self, path: &ScriptPath) -> DomainResult<MigrationScript> {
        self.scripts
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| DomainError::ScriptNotFound(path.as_path().to_path_buf()))
    }

    async fn get_children(&self, _directory_path: &Path) -> DomainResult<Vec<ScriptPath>> {
        Ok(Vec::new())
    }

    async fn get_scripts_after(
        &self,
        _directory: &Path,
        _after: &ScriptPath,
    ) -> DomainResult<Vec<ScriptPath>> {
        Ok(Vec::new())
    }

    async fn get_scripts_after_in_current(
        &self,
        _after_name: &str,
    ) -> DomainResult<Vec<ScriptPath>> {
        Ok(Vec::new())
    }

    async fn get_scripts_in_current(&self) -> DomainResult<Vec<ScriptPath>> {
        Ok(Vec::new())
    }

    async fn get_scripts_after_global(&self, _after_name: &str) -> DomainResult<Vec<ScriptPath>> {
        Ok(Vec::new())
    }
}

/// In-memory fake for `ScriptExecutor`.
///
/// Defaults to succeeding every execution with the script's own checksum.
/// Use `set_result` to script a specific outcome and `executed_paths` to
/// assert on what was actually run.
pub struct FakeScriptExecutor {
    result: Mutex<Option<ExecutionResult>>,
    connection_result: Mutex<DomainResult<()>>,
    executed_paths: Mutex<Vec<ScriptPath>>,
}

impl Default for FakeScriptExecutor {
    fn default() -> Self {
        Self {
            result: Mutex::new(None),
            connection_result: Mutex::new(Ok(())),
            executed_paths: Mutex::new(Vec::new()),
        }
    }
}

#[allow(dead_code)] // Some helpers reserved for tests not yet written
impl FakeScriptExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Script the `ExecutionResult` returned by every subsequent `execute` call.
    pub fn set_result(&self, result: ExecutionResult) {
        *self.result.lock().unwrap() = Some(result);
    }

    /// Script the outcome of `test_connection`.
    pub fn set_connection_result(&self, result: DomainResult<()>) {
        *self.connection_result.lock().unwrap() = result;
    }

    /// Paths passed to `execute`, in call order.
    pub fn executed_paths(&self) -> Vec<ScriptPath> {
        self.executed_paths.lock().unwrap().clone()
    }
}

#[async_trait]
impl ScriptExecutor for FakeScriptExecutor {
    async fn execute(&self, script: &MigrationScript) -> DomainResult<ExecutionResult> {
        self.executed_paths.lock().unwrap().push(script.path.clone());

        let result = self
            .result
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| ExecutionResult::success(0, script.checksum));

        Ok(result)
    }

    async fn test_connection(&self) -> DomainResult<()> {
        match &*self.connection_result.lock().unwrap() {
            Ok(()) => Ok(()),
            Err(e) => Err(DomainError::ExecutionFailed(e.to_string())),
        }
    }
}

/// In-memory fake for `ExecutionTracker`.
///
/// Statuses returned by `get_status` / `get_database_status` can be
/// pre-configured with `set_status`; otherwise they default to `NeverRun`.
/// Recorded executions and skip state are queryable for assertions.
#[derive(Default)]
pub struct FakeExecutionTracker {
    statuses: Mutex<HashMap<String, ScriptStatus>>,
    recorded_executions: Mutex<HashMap<String, ExecutionResult>>,
    skipped: Mutex<HashSet<String>>,
}

#[allow(dead_code)] // Some helpers reserved for tests not yet written
impl FakeExecutionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-configure the status returned for `path` by both `get_status` and
    /// `get_database_status`.
    pub fn set_status(&self, path: &ScriptPath, status: ScriptStatus) {
        self.statuses
            .lock()
            .unwrap()
            .insert(path.to_string(), status);
    }

    /// The execution result last recorded for `path`, if any.
    pub fn recorded_execution(&self, path: &ScriptPath) -> Option<ExecutionResult> {
        self.recorded_executions
            .lock()
            .unwrap()
            .get(&path.to_string())
            .cloned()
    }
}

#[async_trait]
impl ExecutionTracker for FakeExecutionTracker {
    async fn record_execution(
        &self,
        path: &ScriptPath,
        result: &ExecutionResult,
    ) -> DomainResult<()> {
        self.recorded_executions
            .lock()
            .unwrap()
            .insert(path.to_string(), result.clone());
        Ok(())
    }

    async fn get_status(
        &self,
        path: &ScriptPath,
        _current_checksum: Checksum,
    ) -> DomainResult<ScriptStatus> {
        Ok(self
            .statuses
            .lock()
            .unwrap()
            .get(&path.to_string())
            .cloned()
            .unwrap_or(ScriptStatus::NeverRun))
    }

    async fn get_database_status(&self, path: &ScriptPath) -> DomainResult<ScriptStatus> {
        Ok(self
            .statuses
            .lock()
            .unwrap()
            .get(&path.to_string())
            .cloned()
            .unwrap_or(ScriptStatus::NeverRun))
    }

    async fn is_skipped(&self, path: &ScriptPath) -> DomainResult<bool> {
        Ok(self.skipped.lock().unwrap().contains(&path.to_string()))
    }

    async fn mark_skipped(&self, path: &ScriptPath) -> DomainResult<()> {
        self.skipped.lock().unwrap().insert(path.to_string());
        Ok(())
    }

    async fn unmark_skipped(&self, path: &ScriptPath) -> DomainResult<()> {
        self.skipped.lock().unwrap().remove(&path.to_string());
        Ok(())
    }

    async fn get_all_executed_scripts(&self) -> DomainResult<HashSet<String>> {
        Ok(self
            .recorded_executions
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect())
    }
}
