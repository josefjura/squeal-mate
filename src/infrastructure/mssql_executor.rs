//! SQL Server executor implementation

use crate::db::Database;
use crate::domain::{DomainResult, ExecutionResult, MigrationScript, ScriptExecutor};
use crate::infrastructure::error::InfraError;
use async_trait::async_trait;
use std::error::Error;
use tokio::time::Instant;

/// SQL Server implementation of script executor
pub struct MssqlExecutor {
    db: Database,
}

impl MssqlExecutor {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Format SQL error messages for better readability
    fn format_sql_error(error: &dyn Error, script_content: &str) -> String {
        let error_msg = error.to_string();

        // Try to extract line number from common SQL Server error patterns
        // SQL Server errors often include "Line X" in the message
        if let Some(line_num) = Self::extract_line_number(&error_msg) {
            // Get the relevant SQL snippet
            let snippet = Self::get_sql_snippet(script_content, line_num);

            format!(
                "SQL Error at line {}:\n{}\n\nContext:\n{}",
                line_num, error_msg, snippet
            )
        } else {
            // No line number found, just format the error nicely
            format!("SQL Error:\n{}", error_msg)
        }
    }

    /// Extract line number from SQL Server error message
    fn extract_line_number(error_msg: &str) -> Option<usize> {
        // Look for patterns like "Line 5" or "line 5"
        for word in error_msg.split_whitespace() {
            if let Some(stripped) = word.strip_suffix(',') {
                // Handle "Line 5,"
                if let Ok(num) = stripped.parse::<usize>() {
                    return Some(num);
                }
            } else if let Ok(num) = word.parse::<usize>() {
                // Handle "Line 5"
                return Some(num);
            }
        }

        // Also try regex-like pattern matching for "Line X" or "line X"
        if let Some(idx) = error_msg.to_lowercase().find("line ") {
            let after_line = &error_msg[idx + 5..];
            if let Some(num_str) = after_line.split_whitespace().next() {
                // Remove trailing punctuation
                let clean_num = num_str.trim_end_matches(|c: char| !c.is_ascii_digit());
                if let Ok(num) = clean_num.parse::<usize>() {
                    return Some(num);
                }
            }
        }

        None
    }

    /// Get a snippet of SQL around the error line
    fn get_sql_snippet(script: &str, error_line: usize) -> String {
        let lines: Vec<&str> = script.lines().collect();

        if error_line == 0 || error_line > lines.len() {
            return String::from("(Unable to show context)");
        }

        // Show 2 lines before and after the error (if available)
        let start = error_line.saturating_sub(3);
        let end = (error_line + 2).min(lines.len());

        let mut snippet = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_num = start + i + 1;
            if line_num == error_line {
                snippet.push_str(&format!(" >>> {:3} | {}\n", line_num, line));
            } else {
                snippet.push_str(&format!("     {:3} | {}\n", line_num, line));
            }
        }

        snippet
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
            Err(e) => {
                // Format the error with context and line numbers
                let formatted_error = Self::format_sql_error(e.as_ref(), content);
                Ok(ExecutionResult::failure(
                    formatted_error,
                    elapsed,
                    script.checksum,
                ))
            }
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
