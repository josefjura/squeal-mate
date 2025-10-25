//! SQL Server executor implementation

use crate::batch_parser::BatchParser;
use crate::db::Database;
use crate::domain::{DomainResult, ExecutionResult, MigrationScript, ScriptExecutor};
use crate::infrastructure::error::InfraError;
use async_trait::async_trait;
use tokio::time::Instant;

/// SQL Server implementation of script executor
pub struct MssqlExecutor {
    db: Database,
}

impl MssqlExecutor {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ScriptExecutor for MssqlExecutor {
    async fn execute(&self, script: &MigrationScript) -> DomainResult<ExecutionResult> {
        let start = Instant::now();

        // Remove BOM if present
        let mut content = script.content.as_str();
        if content.starts_with('\u{feff}') {
            content = &content[3..];
        }

        // Execute the script
        let result = self.db.execute_script(content).await;
        let elapsed = start.elapsed().as_millis();

        match result {
            Ok(_) => Ok(ExecutionResult::success(elapsed, script.checksum)),
            Err(e) => Ok(ExecutionResult::failure(
                e.to_string(),
                elapsed,
                script.checksum,
            )),
        }
    }

    async fn test_connection(&self) -> DomainResult<()> {
        // For now, we'll just try to execute a simple query
        // The existing Database doesn't have a dedicated test method
        // We could add one, but for now this will work

        self.db
            .execute_script("SELECT 1")
            .await
            .map_err(|e| InfraError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
