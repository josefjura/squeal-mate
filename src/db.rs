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
}

#[derive(Debug, Clone)]
pub enum Authentication {
    Integrated,
    SqlServer { username: String, password: String },
}

impl Database {
    pub async fn execute_script(
        &self,
        mut script: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        //let mut script = tokio::fs::read_to_string(path).await?;
        if script.starts_with('\u{feff}') {
            script = &script[3..];
        }

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

        config.trust_cert();

        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;

        let mut client = Client::connect(config, tcp.compat_write()).await?;

        let parse = BatchParser::parse(&script);

        for batch in parse.batches {
            client.simple_query(batch).await?;
        }

        Ok(())
    }
}
