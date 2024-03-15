use std::collections::HashMap;

use clap::{Args, Parser, Subcommand};

use crate::{db::Database, ArgumentsError};

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
    pub port: Option<String>,
    /// Username used to log into db
    #[arg(long, short = 'u')]
    pub username: Option<String>,
    /// Password used to log into db
    #[arg(long, short = 'p')]
    pub password: Option<String>,
}

impl ConnectionArgs {
    pub fn merge(
        self: &ConnectionArgs,
        from_config: &HashMap<String, String>,
    ) -> Result<Database, ArgumentsError> {
        const DEFAULT_SERVER: &str = "localhost";
        const DEFAULT_PORT: &str = "1433";

        let server = self
            .server
            .clone()
            .or_else(|| from_config.get("server").cloned())
            .unwrap_or_else(|| DEFAULT_SERVER.to_owned());

        let port_str = self
            .port
            .clone()
            .or_else(|| from_config.get("port").cloned())
            .unwrap_or_else(|| DEFAULT_PORT.to_owned());

        let port = port_str
            .parse::<u16>()
            .map_err(|_| ArgumentsError::PortNotNumber)?;

        let username = self
            .username
            .clone()
            .or_else(|| from_config.get("username").cloned())
            .ok_or(ArgumentsError::MissingUsername)?;

        let password = self
            .password
            .clone()
            .or_else(|| from_config.get("password").cloned())
            .ok_or(ArgumentsError::MissingPassword)?;

        Ok(Database {
            server,
            port,
            username,
            password,
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
