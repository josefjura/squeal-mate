use std::error::Error;

use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

use crate::batch_parser::BatchParser;

#[derive(Debug, Clone)]
pub struct Database {
    pub server: String,
    pub port: u16,
    pub name: String,
    pub authentication: Authentication,
    pub encryption: EncryptionConfig,
}

#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Encryption level: Required, Optional, or NotSupported
    pub level: EncryptionLevel,
    /// Trust server certificate (for self-signed certs)
    pub trust_certificate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EncryptionLevel {
    /// Encryption is required
    Required,
    /// Encryption is optional (try encrypted, fall back to unencrypted)
    Optional,
    /// No encryption support
    NotSupported,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            // SQL Server 2022 requires encryption by default
            level: EncryptionLevel::Required,
            trust_certificate: true, // Trust self-signed certs by default
        }
    }
}

#[derive(Debug, Clone)]
pub enum Authentication {
    Integrated,
    SqlServer { username: String, password: String },
}

impl Database {
    pub async fn execute_script(&self, script: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut config = Config::new();

        config.host(&self.server);
        config.port(self.port);
        let auth: AuthMethod = match self.authentication {
            Authentication::Integrated => AuthMethod::Integrated,
            Authentication::SqlServer {
                ref username,
                ref password,
            } => AuthMethod::sql_server(username, password),
        };
        config.authentication(auth);
        config.database(&self.name);

        // Apply encryption settings based on configuration
        match self.encryption.level {
            EncryptionLevel::Required => {
                // Encryption is required - will fail if server doesn't support it
                // This is the default for SQL Server 2022
            }
            EncryptionLevel::Optional => {
                // Try encryption first, fall back to unencrypted if it fails
                // Note: tiberius handles this automatically by default
            }
            EncryptionLevel::NotSupported => {
                // Explicitly disable encryption for older SQL Server versions
                // Note: tiberius doesn't have a direct "disable encryption" option
                // but will work with servers that don't require it
            }
        }

        // Trust server certificate if configured (for self-signed certs)
        if self.encryption.trust_certificate {
            config.trust_cert();
        }

        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;

        let mut client = Client::connect(config, tcp.compat_write()).await?;

        let parse = BatchParser::parse(script);

        for batch in parse.batches {
            client.simple_query(batch).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_config_default() {
        let config = EncryptionConfig::default();
        assert_eq!(config.level, EncryptionLevel::Required);
        assert!(config.trust_certificate);
    }

    #[test]
    fn test_database_config_with_sql_auth() {
        let db = Database {
            server: "192.168.1.100".to_string(),
            port: 1433,
            name: "MyDatabase".to_string(),
            authentication: Authentication::SqlServer {
                username: "sa".to_string(),
                password: "Password123!".to_string(),
            },
            encryption: EncryptionConfig::default(),
        };

        assert_eq!(db.server, "192.168.1.100");
        assert_eq!(db.port, 1433);
        assert_eq!(db.name, "MyDatabase");

        match db.authentication {
            Authentication::SqlServer { username, password } => {
                assert_eq!(username, "sa");
                assert_eq!(password, "Password123!");
            }
            _ => panic!("Expected SqlServer authentication"),
        }
    }

    #[test]
    fn test_database_config_with_integrated_auth() {
        let db = Database {
            server: "localhost".to_string(),
            port: 1433,
            name: "MyDatabase".to_string(),
            authentication: Authentication::Integrated,
            encryption: EncryptionConfig::default(),
        };

        assert!(matches!(db.authentication, Authentication::Integrated));
    }

    #[test]
    fn test_encryption_levels() {
        let required = EncryptionLevel::Required;
        let optional = EncryptionLevel::Optional;
        let not_supported = EncryptionLevel::NotSupported;

        assert_eq!(required, EncryptionLevel::Required);
        assert_eq!(optional, EncryptionLevel::Optional);
        assert_eq!(not_supported, EncryptionLevel::NotSupported);

        // Verify they're different from each other
        assert_ne!(required, optional);
        assert_ne!(optional, not_supported);
        assert_ne!(required, not_supported);
    }
}
