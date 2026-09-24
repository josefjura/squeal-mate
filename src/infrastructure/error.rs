//! Infrastructure-level errors
//!
//! These errors represent failures in external systems (IO, database, etc.)

use crate::domain::DomainError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InfraError {
    #[error("File system error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("SQL Server error: {0}")]
    TiberiusError(#[from] tiberius::error::Error),

    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("Script database error: {0}")]
    ScriptDatabaseError(#[from] crate::script_memory::ScriptDatabaseError),
}

// Convert infrastructure errors to domain errors where appropriate
impl From<InfraError> for DomainError {
    fn from(error: InfraError) -> Self {
        match error {
            InfraError::IoError(e) => DomainError::ExecutionFailed(e.to_string()),
            InfraError::DatabaseError(e) => DomainError::ExecutionFailed(e),
            InfraError::TiberiusError(e) => DomainError::ExecutionFailed(e.to_string()),
            InfraError::SqliteError(e) => DomainError::ExecutionFailed(e.to_string()),
            InfraError::ScriptDatabaseError(e) => DomainError::ExecutionFailed(e.to_string()),
        }
    }
}
