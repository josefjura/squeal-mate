//! Human-readable troubleshooting guidance for database connection errors

use crate::db::{Authentication, Database, EncryptionLevel};

/// Formats a connection error with detailed troubleshooting guidance
pub fn format_connection_error(error_msg: &str, db_config: &Database) -> String {
    let error_msg = error_msg.to_lowercase();

    // Check for specific error patterns
    if error_msg.contains("connection refused") || error_msg.contains("could not connect") {
        format!(
            r#"❌ Connection refused: SQL Server is not accepting connections

Server: {}:{}
Database: {}

Possible causes:
1. SQL Server is not running
   - Windows: Check "SQL Server (MSSQLSERVER)" service in Services
   - Linux: Check sqlservr process with: ps aux | grep sqlservr
   - Docker: Ensure container is running: docker ps

2. TCP/IP protocol is disabled
   - Open SQL Server Configuration Manager
   - Navigate to: SQL Server Network Configuration > Protocols for MSSQLSERVER
   - Enable "TCP/IP" protocol
   - Restart SQL Server service

3. Firewall is blocking port {}
   - Windows Firewall: Add inbound rule for port {}
   - Linux: sudo ufw allow {}/tcp
   - Check with: telnet {} {}

4. Wrong server address or port
   - Verify server name/IP: {}
   - Default SQL Server port is 1433
   - Check SQL Server Configuration Manager for actual port

To fix:
  1. Check configuration: squealmate config
  2. Reconfigure: squealmate init
"#,
            db_config.server,
            db_config.port,
            db_config.name,
            db_config.port,
            db_config.port,
            db_config.port,
            db_config.server,
            db_config.port,
            db_config.server
        )
    } else if error_msg.contains("tls")
        || error_msg.contains("ssl")
        || error_msg.contains("encryption")
        || error_msg.contains("certificate")
    {
        let encryption_setting = match db_config.encryption.level {
            EncryptionLevel::Required => "required",
            EncryptionLevel::Optional => "optional",
            EncryptionLevel::NotSupported => "not_supported",
        };

        format!(
            r#"❌ Encryption/TLS error

Server: {}:{}
Current encryption setting: {}
Trust certificate: {}

SQL Server 2022 requires encryption by default. This error usually means:
1. Server requires encryption but connection failed
2. Certificate validation failed (self-signed certificate)
3. Outdated TLS protocol

Quick fixes:

Option 1: Trust server certificate (for development/self-signed certs)
  Add to your config file (~/.config/squealmate/config.toml):

  [database]
  encryption = "required"
  trust_server_certificate = true

Option 2: Disable encryption requirement on SQL Server (NOT recommended for production)
  - Open SQL Server Configuration Manager
  - Navigate to: SQL Server Network Configuration > Protocols for MSSQLSERVER
  - Right-click "Protocols" > Properties
  - In "Flags" tab: Set "Force Encryption" to "No"
  - Restart SQL Server

  Then in config:
  [database]
  encryption = "optional"

Option 3: Install valid certificate on SQL Server
  - Use proper CA-signed certificate
  - SQL Server Configuration Manager > Certificate tab
  - Select valid certificate and restart service

To reconfigure: squealmate init
"#,
            db_config.server,
            db_config.port,
            encryption_setting,
            db_config.encryption.trust_certificate
        )
    } else if error_msg.contains("login")
        || error_msg.contains("authentication")
        || error_msg.contains("password")
        || error_msg.contains("user")
    {
        let (username, auth_mode) = match &db_config.authentication {
            Authentication::Integrated => ("[Windows Authentication]".to_string(), "Integrated"),
            Authentication::SqlServer { username, .. } => (username.clone(), "SQL Server"),
        };

        format!(
            r#"❌ Login failed: Invalid credentials or authentication mode

Server: {}:{}
Database: {}
Authentication mode: {}
Username: {}

Troubleshooting:

1. Verify SQL Server authentication mode
   - Right-click server in SSMS > Properties > Security
   - Must be "SQL Server and Windows Authentication mode" for SQL auth
   - Requires SQL Server restart after change

2. Verify user exists and has permissions
   - Connect to SQL Server as admin
   - Run: SELECT name FROM sys.sql_logins WHERE name = '{}';
   - Check database access:
     USE [{}];
     SELECT dp.name, dp.type_desc
     FROM sys.database_principals dp
     WHERE dp.name = '{}';

3. Check user has necessary database roles
   Required roles: db_ddladmin, db_datareader, db_datawriter

   To grant permissions:
   USE [{}];
   ALTER ROLE db_ddladmin ADD MEMBER [{}];
   ALTER ROLE db_datareader ADD MEMBER [{}];
   ALTER ROLE db_datawriter ADD MEMBER [{}];

4. For Windows Authentication (Integrated):
   - Ensure you're running as correct Windows user
   - User must have SQL Server login and database access
   - On Linux: May require Kerberos configuration

To reconfigure: squealmate init
"#,
            db_config.server,
            db_config.port,
            db_config.name,
            auth_mode,
            username,
            username,
            db_config.name,
            username,
            db_config.name,
            username,
            username,
            username
        )
    } else {
        // Generic error
        format!(
            r#"❌ Database connection error

Server: {}:{}
Database: {}

Common fixes:
  1. Check your configuration: squealmate config
  2. Reconfigure database: squealmate init
  3. Verify SQL Server is running
  4. Check network connectivity: ping {}
  5. Test port access: telnet {} {}

If the error persists:
  - Check SQL Server error logs
  - Review Windows Event Viewer (Windows)
  - Check firewall settings
  - Verify database name exists
"#,
            db_config.server,
            db_config.port,
            db_config.name,
            db_config.server,
            db_config.server,
            db_config.port
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::EncryptionConfig;

    fn create_test_db_config() -> Database {
        Database {
            server: "localhost".to_string(),
            port: 1433,
            name: "TestDB".to_string(),
            authentication: Authentication::SqlServer {
                username: "test_user".to_string(),
                password: "test_pass".to_string(),
            },
            encryption: EncryptionConfig::default(),
        }
    }

    #[test]
    fn test_format_connection_refused_error() {
        let config = create_test_db_config();
        let error = format_connection_error("connection refused", &config);

        assert!(error.contains("Connection refused"));
        assert!(error.contains("localhost:1433"));
        assert!(error.contains("TestDB"));
        assert!(error.contains("SQL Server is not running"));
        assert!(error.contains("TCP/IP protocol"));
        assert!(error.contains("Firewall"));
    }

    #[test]
    fn test_format_tls_error() {
        let config = create_test_db_config();
        let error = format_connection_error("tls handshake failed", &config);

        assert!(error.contains("Encryption/TLS error"));
        assert!(error.contains("SQL Server 2022"));
        assert!(error.contains("trust_server_certificate = true"));
        assert!(error.contains("encryption = \"required\""));
    }

    #[test]
    fn test_format_certificate_error() {
        let config = create_test_db_config();
        let error = format_connection_error("certificate validation failed", &config);

        assert!(error.contains("Encryption/TLS error"));
        assert!(error.contains("Certificate validation failed"));
    }

    #[test]
    fn test_format_auth_error() {
        let config = create_test_db_config();
        let error = format_connection_error("login failed for user", &config);

        assert!(error.contains("Login failed")); // Actual message
        assert!(error.contains("test_user"));
        assert!(error.contains("SQL Server authentication mode"));
        assert!(error.contains("database_principals"));
    }

    #[test]
    fn test_format_timeout_error() {
        let config = create_test_db_config();
        let error = format_connection_error("connection timed out", &config);

        // Timeout errors fall through to generic case
        assert!(error.contains("Database connection error"));
        assert!(error.contains("localhost:1433"));
        assert!(error.contains("TestDB"));
    }

    #[test]
    fn test_format_generic_error() {
        let config = create_test_db_config();
        let error = format_connection_error("some unknown error", &config);

        assert!(error.contains("Database connection error"));
        assert!(error.contains("localhost:1433"));
        assert!(error.contains("TestDB"));
        assert!(error.contains("squealmate config"));
    }
}
