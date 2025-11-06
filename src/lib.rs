//! SquealMate - TUI for managing incremental SQL migrations
//!
//! This library exposes the core types and infrastructure for testing.

pub mod action;
pub mod app;
pub mod batch_parser;
pub mod cli;
pub mod db;
pub mod domain;
pub mod entries;
pub mod error;
pub mod infrastructure;
pub mod repository;
pub mod screen;
pub mod script_memory;
pub mod services;
pub mod tui;
pub mod ui;
pub mod utils;

// Re-export commonly needed types
pub use app::{App, AppState};
pub use error::ArgumentsError;
pub use infrastructure::{FileSystem, FsEntry};
