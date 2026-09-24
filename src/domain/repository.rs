//! Repository traits for domain layer
//!
//! These traits define the interface for accessing migration scripts
//! without coupling to infrastructure details.

use crate::domain::error::DomainResult;
use crate::domain::script::{MigrationScript, ScriptPath};
use async_trait::async_trait;

/// Repository for accessing migration scripts from storage
#[async_trait]
pub trait MigrationRepository: Send + Sync {
    /// Read a script's content
    async fn read_script(&self, path: &ScriptPath) -> DomainResult<MigrationScript>;
}
