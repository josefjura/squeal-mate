//! UI layer - Terminal user interface components
//!
//! This module contains thin UI adapters that handle rendering and user input.
//! Components delegate business logic to services.

pub mod component;
pub mod help;
pub mod list;
pub mod list_state;
pub mod script_status;
pub mod scroll_list;

// New unified view components
pub mod unified_view;
pub mod command_bar;
pub mod script_preview;
pub mod execution_log;

pub use component::Component;
pub use unified_view::UnifiedView;
