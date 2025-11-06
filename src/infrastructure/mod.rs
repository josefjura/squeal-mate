//! Infrastructure layer - External system implementations
//!
//! This module contains implementations that interact with external systems:
//! - File system (repository, file explorer)
//! - SQLite (execution tracking)
//! - SQL Server (script execution)
//! - Configuration loading

pub mod config;
pub mod error;
pub mod file_explorer;
pub mod filesystem;
pub mod filesystem_repository;
pub mod mssql_executor;
pub mod sqlite_tracker;

// Re-export commonly used types
pub use config::{get_config_dir, get_data_dir, get_script_database, Settings};
pub use file_explorer::FileExplorer;
pub use filesystem::{Entry as FsEntry, FileSystem};
pub use filesystem_repository::FilesystemRepository;
pub use mssql_executor::MssqlExecutor;
pub use sqlite_tracker::SqliteTracker;
