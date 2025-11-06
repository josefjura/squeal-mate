use std::fmt::Display;
use crate::script_memory::ScriptResult;
use crate::domain::ScriptStatus;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]

pub struct ListEntry {
    pub relative_path: String,
    pub name: String,
    pub selected: bool,
    pub is_directory: bool,
    pub status: EntryStatus,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Hash)]
pub enum EntryStatus {
    NeverStarted,
    Finished(bool),
    Changed,
    Unknown,
    Directory,
    Loading,  // New: file/directory is being processed
    Skipped,  // Marked to be skipped (user-defined, persisted in DB)
}

impl Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

/// Convert from ScriptResult (database result) to EntryStatus (UI status)
impl From<ScriptResult> for EntryStatus {
    fn from(result: ScriptResult) -> Self {
        match result {
            ScriptResult::Success => EntryStatus::Finished(true),
            ScriptResult::Error => EntryStatus::Finished(false),
            ScriptResult::Skipped => EntryStatus::Skipped,
        }
    }
}

/// Convert from ScriptStatus (domain status) to EntryStatus (UI status)
impl From<ScriptStatus> for EntryStatus {
    fn from(status: ScriptStatus) -> Self {
        match status {
            ScriptStatus::NeverRun => EntryStatus::NeverStarted,
            ScriptStatus::UpToDate => EntryStatus::Finished(true),
            ScriptStatus::Modified => EntryStatus::Changed,
            ScriptStatus::Failed { .. } => EntryStatus::Finished(false),
            ScriptStatus::Running => EntryStatus::Unknown,
            ScriptStatus::Skipped => EntryStatus::Skipped,
        }
    }
}
