//! Service layer - Application services
//!
//! This module contains services that coordinate between domain and infrastructure layers.
//! Services orchestrate complex operations and manage application workflow.

pub mod migration_service;

pub use migration_service::MigrationService;
