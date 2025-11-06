//! UI layer - Terminal user interface components
//!
//! This module contains thin UI adapters that handle rendering and user input.
//! Components delegate business logic to services.

pub mod component;
pub mod help;
pub mod list;
pub mod script_status;
pub mod tree_state;

// Unified view components
pub mod command_bar;
pub mod execution_log;
pub mod script_preview;
pub mod unified_view;

pub use component::Component;
pub use unified_view::UnifiedView;
