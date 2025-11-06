//! Domain layer - Core business logic with zero external dependencies
//!
//! This module contains the pure business logic of the application,
//! independent of infrastructure concerns like databases, file systems, or UI.

pub mod error;
pub mod executor;
pub mod repository;
pub mod script;
pub mod script_status;
pub mod tracker;

// Re-export commonly used types
pub use error::{DomainError, DomainResult};
pub use executor::ScriptExecutor;
pub use repository::MigrationRepository;
pub use script::{Checksum, MigrationScript, ScriptPath};
pub use script_status::{ExecutionResult, ScriptStatus};
pub use tracker::ExecutionTracker;
