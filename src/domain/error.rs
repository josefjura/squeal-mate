//! Domain-level errors
//!
//! These errors represent business rule violations and domain-specific failures.
//! They should NOT include infrastructure details (no IO errors, database errors, etc.)

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Script not found: {0}")]
    ScriptNotFound(PathBuf),

    #[error("Invalid script path: {0}")]
    InvalidScriptPath(String),

    #[error("Script content is invalid: {0}")]
    InvalidScriptContent(String),

    #[error("Script has already been executed successfully")]
    ScriptAlreadyExecuted,

    #[error("Script execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Checksum mismatch - script has been modified since last execution")]
    ChecksumMismatch,

    #[error("Invalid migration state transition: {0}")]
    InvalidStateTransition(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
