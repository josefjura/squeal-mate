//! Utility functions for logging, panic handling, etc.

pub mod logging;
pub mod panic;

pub use logging::initialize_logging;
pub use panic::initialize_panic_handler;
