//! Infrastructure layer - External system implementations
//!
//! This module contains implementations that interact with external systems:
//! - File system (repository)
//! - SQLite (execution tracking)
//! - SQL Server (script execution)
//! - Configuration loading

pub mod config;
pub mod error;
pub mod filesystem_repository;
pub mod mssql_executor;
pub mod sqlite_tracker;

// Re-export commonly used types
pub use config::{get_config_dir, get_data_dir, get_script_database, Settings};
pub use error::{InfraError, InfraResult};
pub use filesystem_repository::FilesystemRepository;
pub use mssql_executor::MssqlExecutor;
pub use sqlite_tracker::SqliteTracker;
