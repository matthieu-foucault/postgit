use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use git_repository::bstr::ByteSlice;
use git_repository::ObjectId;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use tokio_postgres::NoTls;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Migrate from source to target postgres schemas
#[derive(Args)]
pub struct DiffArgs {
    /// Path to the root of the git repository
    #[arg(long, short, default_value = ".")]
    pub repo_path: String,

    /// Git ref where the source schema can be found
    #[arg(long, short)]
    pub from: String,

    /// Git ref where the target schema can be found
    #[arg(long, short)]
    pub to: String,

    /// Path to the source schema at the source ref, if different from the target path
    #[arg(long)]
    pub source_path: Option<String>,

    /// Path to the schema
    pub path: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Shows the migration diff between two schemas
    Diff(DiffArgs),
}

#[derive(Deserialize)]
struct PostgresConfig {
    user: Option<String>,
    dbname: Option<String>,
    host: Option<String>,
    port: Option<u16>,
}

impl PostgresConfig {
    fn to_tokio_postgres_config(&self) -> tokio_postgres::Config {
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

    fn to_url(&self) -> String {
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
struct DiffEngineConfig {
    source: PostgresConfig,
    target: PostgresConfig,
}

#[derive(Deserialize)]
struct Config {
    diff_engine: DiffEngineConfig,
    target: PostgresConfig,
}

pub fn run(args: &DiffArgs) -> Result<()> {
    let source_path = match &args.source_path {
        Some(path) => path,
        None => &args.path,
    };

    let source_schema = get_schema_script(&args.repo_path, &args.from, source_path)?;
    let target_schema = get_schema_script(&args.repo_path, &args.to, &args.path)?;

    let config = get_config()?;
    let diff_source_tokio_config = config.diff_engine.source.to_tokio_postgres_config();
    let diff_target_tokio_config = config.diff_engine.target.to_tokio_postgres_config();

    create_db(&diff_source_tokio_config)?;
    create_db(&diff_target_tokio_config)?;
    run_sql_script(&source_schema, &diff_source_tokio_config)?;
    run_sql_script(&target_schema, &diff_target_tokio_config)?;

    let output = Command::new("migra")
        .arg(config.diff_engine.source.to_url())
        .arg(config.diff_engine.target.to_url())
        .output()?;

    drop_db(&diff_source_tokio_config)?;
    drop_db(&diff_target_tokio_config)?;

    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    println!("{}", String::from_utf8_lossy(&output.stdout).trim());

    Ok(())
}

fn get_config() -> Result<Config> {
    let mut file = File::open("./config.toml")?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    Ok(toml::from_str(s.as_str())?)
}

fn get_schema_script(repo_path: &str, ref_or_sha1: &str, schema_path: &str) -> Result<String> {
    let repo_path = Path::new(repo_path);
    let schema_path = Path::new(schema_path);

    let repo = git_repository::open(repo_path)?;

    let oid = ObjectId::from_str(ref_or_sha1)?;
    let object_option = repo.try_find_object(oid)?;
    if let Some(object) = object_option {
        let commit = object.try_into_commit()?;
        let tree = commit.tree()?;
        if let Some(entry) = tree.lookup_entry_by_path(schema_path)? {
            let data = &entry.object()?.data;
            Ok(String::from(data.to_str()?))
        } else {
            bail!("Couldn't find entry at path {}", schema_path.display());
        }
    } else {
        bail!("Didn't find source commit for ref {}", ref_or_sha1);
    }
}

#[tokio::main]
async fn create_db(config: &tokio_postgres::Config) -> Result<()> {
    let mut parent_config = config.clone();
    parent_config.dbname("postgres");

    let (client, connection) = parent_config.connect(NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!("create database {}", config.get_dbname().unwrap());
    client.batch_execute(&query).await?;

    Ok(())
}

#[tokio::main]
async fn drop_db(config: &tokio_postgres::Config) -> Result<()> {
    let mut parent_config = config.clone();
    parent_config.dbname("postgres");

    let (client, connection) = parent_config.connect(NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!("drop database {}", config.get_dbname().unwrap());
    client.batch_execute(&query).await?;

    Ok(())
}

#[tokio::main]
async fn run_sql_script(script: &str, config: &tokio_postgres::Config) -> Result<()> {
    // Connect to the database.
    let (client, connection) = config.connect(NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Now we can execute a simple statement that just returns its parameter.
    client.batch_execute(script).await?;

    Ok(())
}
