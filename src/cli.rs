use clap::{Args, Parser, Subcommand};

use crate::{
    config::Settings,
    db::{Authentication, Database},
    ArgumentsError,
};

#[derive(Parser, Debug)]
#[command(
    name = "🦀 Aequitas Command And Control Console 🦀",
    version,
    about,
    long_about = "Support tools collection for the Aequitas team"
)]
pub struct AeqArgs {
    #[clap(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub connection: ConnectionArgs,
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
            .clone()
            .or_else(|| settings.database.port)
            .unwrap_or_else(|| DEFAULT_PORT.to_owned());

        let name = self
            .name
            .clone()
            .or_else(|| settings.database.name.clone())
            .ok_or(ArgumentsError::MissingDBName)?;

        let is_integrated = self
            .is_integrated
            .clone()
            .or_else(|| settings.database.integrated)
            .unwrap_or_else(|| false);

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

        Ok(Database {
            server,
            port,
            name,
            authentication,
        })
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Shows application info and configuration for the current system
    Config,
    /// Starts the migrations explorer
    Migrations,
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
    }) = database
    {
        assert_eq!("test", username);
        assert_eq!("password", password);
        assert_eq!("db_name", name);
    } else {
        panic!("simple_positive: Cannot parse correct result");
    }
}
