//! Migration script domain types
//!
//! This module contains the core domain model for migration scripts.
//! These types are independent of infrastructure concerns.

use crate::domain::error::{DomainError, DomainResult};
use crc::{Crc, CRC_32_ISO_HDLC};
use std::fmt;
use std::path::{Path, PathBuf};

/// A path to a migration script (value object with validation)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptPath(PathBuf);

impl ScriptPath {
    /// Create a new ScriptPath, validating it's a .sql file
    pub fn new(path: impl Into<PathBuf>) -> DomainResult<Self> {
        let path = path.into();

        // Must have .sql extension
        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            return Err(DomainError::InvalidScriptPath(
                format!("Script must have .sql extension: {}", path.display())
            ));
        }

        // Must not be hidden (start with . or _)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name.starts_with('_') {
                return Err(DomainError::InvalidScriptPath(
                    format!("Script filename cannot start with . or _: {}", name)
                ));
            }
        }

        Ok(Self(path))
    }

    /// Create from a trusted path (skips validation - use only for internal conversions)
    pub(crate) fn from_trusted(path: PathBuf) -> Self {
        Self(path)
    }

    /// Get the underlying path
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Get the path as a string slice (if valid UTF-8)
    pub fn as_str(&self) -> Option<&str> {
        self.0.to_str()
    }

    /// Get the filename as a string
    pub fn filename(&self) -> Option<&str> {
        self.0.file_name().and_then(|n| n.to_str())
    }

    /// Convert to PathBuf
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }
}

impl fmt::Display for ScriptPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl AsRef<Path> for ScriptPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// CRC32 checksum (value object)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Checksum(u32);

impl Checksum {
    /// Calculate checksum from content
    pub fn from_content(content: &str) -> Self {
        let hasher = Crc::<u32>::new(&CRC_32_ISO_HDLC);
        Self(hasher.checksum(content.as_bytes()))
    }

    /// Create from a known value (for deserialization)
    pub fn from_value(value: u32) -> Self {
        Self(value)
    }

    /// Get the raw value
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}

/// A migration script with its content and metadata
#[derive(Debug, Clone)]
pub struct MigrationScript {
    pub path: ScriptPath,
    pub content: String,
    pub checksum: Checksum,
}

impl MigrationScript {
    /// Create a new migration script
    pub fn new(path: ScriptPath, content: String) -> Self {
        let checksum = Checksum::from_content(&content);
        Self {
            path,
            content,
            checksum,
        }
    }

    /// Validate that the script content is not empty
    pub fn validate(&self) -> DomainResult<()> {
        if self.content.trim().is_empty() {
            return Err(DomainError::InvalidScriptContent(
                "Script content cannot be empty".to_string()
            ));
        }
        Ok(())
    }

    /// Check if this script has been modified since the given checksum
    pub fn has_changed_since(&self, previous_checksum: Checksum) -> bool {
        self.checksum != previous_checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_path_valid() {
        let path = ScriptPath::new("migration.sql");
        assert!(path.is_ok());
    }

    #[test]
    fn script_path_invalid_extension() {
        let path = ScriptPath::new("migration.txt");
        assert!(path.is_err());
    }

    #[test]
    fn script_path_hidden_file() {
        let path = ScriptPath::new(".hidden.sql");
        assert!(path.is_err());

        let path = ScriptPath::new("_temp.sql");
        assert!(path.is_err());
    }

    #[test]
    fn checksum_from_content() {
        let content = "SELECT * FROM users;";
        let checksum1 = Checksum::from_content(content);
        let checksum2 = Checksum::from_content(content);

        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn checksum_different_content() {
        let checksum1 = Checksum::from_content("SELECT 1;");
        let checksum2 = Checksum::from_content("SELECT 2;");

        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn migration_script_validate_empty() {
        let path = ScriptPath::new("test.sql").unwrap();
        let script = MigrationScript::new(path, "   \n  ".to_string());

        assert!(script.validate().is_err());
    }

    #[test]
    fn migration_script_validate_valid() {
        let path = ScriptPath::new("test.sql").unwrap();
        let script = MigrationScript::new(path, "SELECT 1;".to_string());

        assert!(script.validate().is_ok());
    }

    #[test]
    fn migration_script_has_changed() {
        let path = ScriptPath::new("test.sql").unwrap();
        let script1 = MigrationScript::new(path.clone(), "SELECT 1;".to_string());
        let script2 = MigrationScript::new(path, "SELECT 2;".to_string());

        assert!(script2.has_changed_since(script1.checksum));
        assert!(!script1.has_changed_since(script1.checksum));
    }
}
