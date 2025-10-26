//! UI layer - Terminal user interface components
//!
//! This module contains thin UI adapters that handle rendering and user input.
//! Components delegate business logic to services.

pub mod component;
pub mod help;
pub mod list;
pub mod script_status;
pub mod scroll_list;

pub use component::Component;
