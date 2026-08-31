//! Script status and execution result types

use crate::domain::script::Checksum;
use std::fmt;

/// The execution status of a migration script
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptStatus {
    /// Script has never been run
    NeverRun,

    /// Script is up-to-date (executed successfully with matching checksum)
    UpToDate,

    /// Script has been modified since last execution
    Modified,

    /// Script execution failed
    Failed { error: String },

    /// Script is marked to be skipped (user-defined)
    Skipped,
}

impl ScriptStatus {
    /// Determine status based on execution history
    pub fn from_execution_history(
        has_been_executed: bool,
        last_execution_succeeded: bool,
        current_checksum: Checksum,
        stored_checksum: Option<Checksum>,
    ) -> Self {
        match (has_been_executed, stored_checksum) {
            (false, _) => Self::NeverRun,
            (true, None) => Self::NeverRun, // No checksum stored, treat as never run
            (true, Some(stored)) if stored != current_checksum => Self::Modified,
            (true, Some(_)) if last_execution_succeeded => Self::UpToDate,
            (true, Some(_)) => Self::Failed {
                error: "Previous execution failed".to_string(),
            },
        }
    }
}

impl fmt::Display for ScriptStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NeverRun => write!(f, "Never Run"),
            Self::UpToDate => write!(f, "Up to Date"),
            Self::Modified => write!(f, "Modified"),
            Self::Failed { error } => write!(f, "Failed: {}", error),
            Self::Skipped => write!(f, "Skipped"),
        }
    }
}

/// Result of script execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub error: Option<String>,
    pub elapsed_ms: u128,
    pub checksum: Checksum,
}

impl ExecutionResult {
    pub fn success(elapsed_ms: u128, checksum: Checksum) -> Self {
        Self {
            success: true,
            error: None,
            elapsed_ms,
            checksum,
        }
    }

    pub fn failure(error: String, elapsed_ms: u128, checksum: Checksum) -> Self {
        Self {
            success: false,
            error: Some(error),
            elapsed_ms,
            checksum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_never_run() {
        let checksum = Checksum::from_value(123);
        let status = ScriptStatus::from_execution_history(false, false, checksum, None);

        assert_eq!(status, ScriptStatus::NeverRun);
    }

    #[test]
    fn status_up_to_date() {
        let checksum = Checksum::from_value(123);
        let status = ScriptStatus::from_execution_history(true, true, checksum, Some(checksum));

        assert_eq!(status, ScriptStatus::UpToDate);
    }

    #[test]
    fn status_modified() {
        let old_checksum = Checksum::from_value(123);
        let new_checksum = Checksum::from_value(456);
        let status =
            ScriptStatus::from_execution_history(true, true, new_checksum, Some(old_checksum));

        assert_eq!(status, ScriptStatus::Modified);
    }

    #[test]
    fn status_failed() {
        let checksum = Checksum::from_value(123);
        let status = ScriptStatus::from_execution_history(true, false, checksum, Some(checksum));

        assert!(matches!(status, ScriptStatus::Failed { .. }));
    }
}
