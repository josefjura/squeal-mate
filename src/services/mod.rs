//! Service layer - Application services
//!
//! This module contains services that coordinate between domain and infrastructure layers.
//! Services orchestrate complex operations and manage application workflow.

pub mod action_dispatcher;
pub mod migration_service;

pub use action_dispatcher::ActionDispatcher;
pub use migration_service::MigrationService;
