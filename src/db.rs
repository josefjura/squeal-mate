use std::{error::Error, path::PathBuf};

use tiberius::{AuthMethod, Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

#[derive(Debug)]
pub struct Database {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl Database {
    pub async fn execute_script(&self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let mut script = tokio::fs::read_to_string(path).await?;

        if script.starts_with("\u{feff}") {
            script = script[3..].to_string();
        }

        let mut config = Config::new();

        config.host(&self.server);
        config.port(self.port);
        config.authentication(AuthMethod::sql_server(&self.username, &self.password));
        config.trust_cert();

        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;

        let mut client = Client::connect(config, tcp.compat_write()).await?;

        client.simple_query(script).await?;

        Ok(())
    }
}
