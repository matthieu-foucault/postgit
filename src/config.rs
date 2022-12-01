use anyhow::Result;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct PostgresConfig {
    #[serde(default = "default_user")]
    user: String,
    #[serde(default = "default_db")]
    dbname: String,
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_user() -> String {
    env::var("PGUSER").unwrap_or_else(|_| "postgres".to_string())
}

fn default_db() -> String {
    env::var("PGDATABASE").unwrap_or_else(|_| default_user())
}

fn default_host() -> String {
    env::var("PGHOST").unwrap_or_else(|_| "localhost".to_string())
}

fn default_port() -> u16 {
    env::var("PGPORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(5432)
}

impl PostgresConfig {
    pub fn to_tokio_postgres_config(&self) -> tokio_postgres::Config {
        let mut config = tokio_postgres::Config::new();

        config.host(&self.host);
        config.user(&self.user);
        config.dbname(&self.dbname);
        config.port(self.port);

        config
    }

    pub fn to_url(&self) -> String {
        let mut url = "postgresql://".to_owned();

        url.push_str(&self.user);
        url.push('@');

        url.push_str(&self.host);

        url.push(':');
        url.push_str(&self.port.to_string());

        url.push('/');
        url.push_str(&self.dbname);

        url
    }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        PostgresConfig {
            user: default_user(),
            dbname: default_db(),
            host: default_host(),
            port: default_port(),
        }
    }
}

#[derive(Deserialize, PartialEq, Eq, Debug, Default)]
pub struct DiffEngineConfig {
    pub command: Option<String>,
    #[serde(default)]
    pub source: PostgresConfig,
    #[serde(default)]
    pub target: PostgresConfig,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct Config {
    #[serde(default)]
    pub diff_engine: DiffEngineConfig,
    #[serde(default)]
    pub target: PostgresConfig,
}

impl Config {
    pub fn build() -> Result<Config> {
        let config_path = Path::new("postgit.toml");
        let mut s = String::new();
        if config_path.try_exists()? {
            let mut file = File::open(config_path)?;
            file.read_to_string(&mut s)?;
        }

        let mut config: Config = toml::from_str(s.as_str())?;
        let default_db = default_db();

        if config.diff_engine.source.dbname == default_db {
            config.diff_engine.source.dbname = "postgit_diff_source".to_string();
        }

        if config.diff_engine.target.dbname == default_db {
            config.diff_engine.target.dbname = "postgit_diff_target".to_string();
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env::set_current_dir, fs};
    use tempfile::tempdir;

    #[test]
    fn it_loads_the_default_config() {
        let dir = tempdir().unwrap();
        set_current_dir(&dir).unwrap();
        let config = Config::build().unwrap();

        assert_eq!(
            Config {
                diff_engine: DiffEngineConfig {
                    command: None,
                    source: PostgresConfig {
                        user: "postgres".to_string(),
                        dbname: "postgit_diff_source".to_string(),
                        host: "localhost".to_string(),
                        port: 5432
                    },
                    target: PostgresConfig {
                        user: "postgres".to_string(),
                        dbname: "postgit_diff_target".to_string(),
                        host: "localhost".to_string(),
                        port: 5432
                    }
                },
                target: PostgresConfig {
                    user: "postgres".to_string(),
                    dbname: "postgres".to_string(),
                    host: "localhost".to_string(),
                    port: 5432
                }
            },
            config
        );
    }

    #[test]
    fn it_loads_the_config_from_file() {
        let dir = tempdir().unwrap();
        println!("{}", dir.path().display());
        let file_path = dir.path().join("postgit.toml");
        println!("{}", file_path.display());
        set_current_dir(&dir).unwrap();
        fs::write(
            &file_path,
            r#"
        [diff_engine]
        command='my_command'

        [diff_engine.source]
        dbname='diff_source_db'
        host='diff_source_host'
        port=1234
        user='diff_source_user'

        [diff_engine.target]
        dbname='diff_target_db'
        host='diff_target_host'
        port=4567
        user='diff_target_user'

        [target]
        dbname='target_db'
        host='target_host'
        port=3214
        user='target_user'
        "#,
        )
        .unwrap();

        let config = Config::build().unwrap();

        assert_eq!(
            Config {
                diff_engine: DiffEngineConfig {
                    command: Some("my_command".to_string()),
                    source: PostgresConfig {
                        user: "diff_source_user".to_string(),
                        dbname: "diff_source_db".to_string(),
                        host: "diff_source_host".to_string(),
                        port: 1234
                    },
                    target: PostgresConfig {
                        user: "diff_target_user".to_string(),
                        dbname: "diff_target_db".to_string(),
                        host: "diff_target_host".to_string(),
                        port: 4567
                    }
                },
                target: PostgresConfig {
                    user: "target_user".to_string(),
                    dbname: "target_db".to_string(),
                    host: "target_host".to_string(),
                    port: 3214
                }
            },
            config
        );
    }
}
