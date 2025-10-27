use clap::{Args, Parser, Subcommand};

use crate::{
    infrastructure::Settings,
    db::{Authentication, Database, EncryptionConfig, EncryptionLevel},
    ArgumentsError,
};

#[derive(Parser, Debug)]
#[command(
    name = "🦀 SquealMate 🦀",
    version,
    about,
    long_about = "Migration management tool for SQL Server"
)]
pub struct SquealMateArgs {
    #[clap(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub connection: ConnectionArgs,

    /// Force start the application even if database connection fails
    #[arg(long, short = 'f')]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct ConnectionArgs {
    /// Database server URL (defaults to localhost)
    #[arg(long, short)]
    pub server: Option<String>,
    /// Port number (defaults to 1433)
    #[arg(long)]
    pub port: Option<u16>,
    /// Username used to log into db
    #[arg(long, short = 'u')]
    pub username: Option<String>,
    /// Password used to log into db
    #[arg(long, short = 'p')]
    pub password: Option<String>,
    /// Name of the database to connect to
    #[arg(long, short = 'n')]
    pub name: Option<String>,
    /// Use integrated authentication. Skips username and password.
    #[arg(long, short = 'i')]
    pub is_integrated: Option<bool>,
}

impl ConnectionArgs {
    pub fn merge(self: &ConnectionArgs, settings: &Settings) -> Result<Database, ArgumentsError> {
        const DEFAULT_SERVER: &str = "localhost";
        const DEFAULT_PORT: u16 = 1433;

        let server = self
            .server
            .clone()
            .or_else(|| settings.database.server.clone())
            .unwrap_or_else(|| DEFAULT_SERVER.to_owned());

        let port = self
            .port
            .or(settings.database.port)
            .unwrap_or(DEFAULT_PORT.to_owned());

        let name = self
            .name
            .clone()
            .or_else(|| settings.database.name.clone())
            .ok_or(ArgumentsError::MissingDBName)?;

        let is_integrated = self
            .is_integrated
            .or(settings.database.integrated)
            .unwrap_or(false);

        let authentication = if is_integrated {
            Authentication::Integrated
        } else {
            let username = self
                .username
                .clone()
                .or_else(|| settings.database.username.clone())
                .ok_or(ArgumentsError::MissingUsername)?;

            let password = self
                .password
                .clone()
                .or_else(|| settings.database.password.clone())
                .ok_or(ArgumentsError::MissingPassword)?;

            Authentication::SqlServer { username, password }
        };

        // Parse encryption settings from config
        let encryption_level = settings
            .database
            .encryption
            .as_ref()
            .map(|s| match s.as_str() {
                "required" => EncryptionLevel::Required,
                "optional" => EncryptionLevel::Optional,
                "not_supported" => EncryptionLevel::NotSupported,
                _ => EncryptionLevel::Required, // Default to required for SQL Server 2022
            })
            .unwrap_or(EncryptionLevel::Required);

        let trust_certificate = settings
            .database
            .trust_server_certificate
            .unwrap_or(true); // Default to true for self-signed certs

        let encryption = EncryptionConfig {
            level: encryption_level,
            trust_certificate,
        };

        Ok(Database {
            server,
            port,
            name,
            authentication,
            encryption,
        })
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Shows application info and configuration for the current system
    Config,
    /// Starts the migrations explorer
    Migrations,
    /// Helps set up the config file
    #[command(name = "init")]
    Initialize,
}

#[test]
fn missing_password() {
    let mut setting = Settings::default();
    setting.database.username = Some("test".to_string());

    let conn = ConnectionArgs {
        is_integrated: None,
        password: None,
        port: None,
        server: None,
        username: None,
        name: None,
    };

    let database = conn.merge(&setting);

    assert!(database.is_err())
}

#[test]
fn missing_username() {
    let setting = Settings::default();

    let conn = ConnectionArgs {
        is_integrated: None,
        password: Some("password".to_string()),
        port: None,
        server: None,
        username: None,
        name: None,
    };

    let database = conn.merge(&setting);

    assert!(database.is_err())
}

#[test]
fn simple_positive() {
    let mut setting = Settings::default();
    setting.database.username = Some("test".to_string());

    let conn = ConnectionArgs {
        is_integrated: None,
        password: Some("password".to_string()),
        port: None,
        server: None,
        username: None,
        name: Some("db_name".to_string()),
    };

    let database = conn.merge(&setting);

    assert!(database.is_ok());

    if let Ok(Database {
        server: _,
        port: _,
        name,
        authentication: Authentication::SqlServer { username, password },
        encryption: _,
    }) = database
    {
        assert_eq!("test", username);
        assert_eq!("password", password);
        assert_eq!("db_name", name);
    } else {
        panic!("simple_positive: Cannot parse correct result");
    }
}
