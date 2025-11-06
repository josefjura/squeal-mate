//! Test support utilities for E2E testing
//!
//! This module provides mocks, builders, and test harnesses for testing
//! SquealMate without real filesystem or database I/O.

pub mod app_builder;
pub mod mock_filesystem;

pub use app_builder::AppBuilder;
pub use mock_filesystem::MockFileSystem;
