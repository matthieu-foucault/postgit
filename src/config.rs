use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Deserialize, Clone)]
pub struct PostgresConfig {
    user: Option<String>,
    dbname: Option<String>,
    host: Option<String>,
    port: Option<u16>,
}

impl PostgresConfig {
    pub fn to_tokio_postgres_config(&self) -> tokio_postgres::Config {
        let mut config = tokio_postgres::Config::new();

        if let Some(host) = &self.host {
            config.host(host);
        }

        if let Some(user) = &self.user {
            config.user(user);
        }

        if let Some(dbname) = &self.dbname {
            config.dbname(dbname);
        }

        if let Some(port) = self.port {
            config.port(port);
        }

        config
    }

    pub fn to_url(&self) -> String {
        let mut url = "postgresql://".to_owned();

        if let Some(user) = &self.user {
            url.push_str(user);
            url.push('@')
        }

        if let Some(host) = &self.host {
            url.push_str(host);
        } else {
            url.push_str("localhost");
        }

        if let Some(port) = self.port {
            url.push(':');
            url.push_str(&port.to_string());
        }

        if let Some(dbname) = &self.dbname {
            url.push('/');
            url.push_str(dbname);
        }

        url
    }
}

#[derive(Deserialize)]
pub struct DiffEngineConfig {
    pub command: Option<String>,
    pub source: PostgresConfig,
    pub target: PostgresConfig,
}

#[derive(Deserialize)]
pub struct Config {
    pub diff_engine: DiffEngineConfig,
    pub target: PostgresConfig,
}

impl Config {
    pub fn build() -> Result<Config> {
        let mut file = File::open("./config.toml")?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        Ok(toml::from_str(s.as_str())?)
    }
}
