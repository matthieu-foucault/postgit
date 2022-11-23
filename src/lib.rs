use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use git_repository::bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::traverse::tree::Recorder;
use git_repository::ObjectId;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;
use tokio_postgres::NoTls;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct DiffArgs {
    /// Path to the root of the git repository
    #[arg(long, short, default_value = ".")]
    pub repo_path: String,

    /// Git ref where the source schema can be found.
    /// This may be omitted for the first migration, when the database is empty
    #[arg(long, short)]
    pub from: Option<String>,

    /// Git ref where the target schema can be found
    #[arg(long, short)]
    pub to: String,

    /// Path to the source schema at the source ref, if different from the target path
    #[arg(long)]
    pub source_path: Option<String>,

    /// Path to the schema file or directory, relative to the repo root
    pub path: String,
}

#[derive(Args)]
pub struct WatchArgs {
    /// Path to the directory to watch
    pub path: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Shows the migration diff between two schemas
    Diff(DiffArgs),
    /// Calculates the migration diff between two schemas and applies it to the target database
    Push(DiffArgs),
    /// Watches a directory and applies the migrations to the target database
    Watch(WatchArgs),
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
pub struct Config {
    diff_engine: DiffEngineConfig,
    target: PostgresConfig,
}

impl Config {
    pub fn build() -> Result<Config> {
        let mut file = File::open("./config.toml")?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        Ok(toml::from_str(s.as_str())?)
    }
}

pub fn apply_diff(args: &DiffArgs, config: &Config) -> Result<()> {
    let diff_string = get_diff_string(args, config)?;
    let target_tokio_config = config.target.to_tokio_postgres_config();
    run_sql_script(&diff_string, &target_tokio_config)
}

pub fn get_diff_string(args: &DiffArgs, config: &Config) -> Result<String> {
    let source_path = match &args.source_path {
        Some(path) => path,
        None => &args.path,
    };

    let source_schema_option = match &args.from {
        Some(from) => Some(get_schema_script(&args.repo_path, from, source_path)?),
        None => None,
    };

    let target_schema = get_schema_script(&args.repo_path, &args.to, &args.path)?;

    let diff_source_tokio_config = config.diff_engine.source.to_tokio_postgres_config();
    let diff_target_tokio_config = config.diff_engine.target.to_tokio_postgres_config();

    // drop temporary dbs in case they were left over from a previous run
    drop_db(&diff_source_tokio_config)?;
    drop_db(&diff_target_tokio_config)?;

    create_db(&diff_source_tokio_config)?;
    create_db(&diff_target_tokio_config)?;
    if let Some(source_schema) = source_schema_option {
        run_sql_script(&source_schema, &diff_source_tokio_config)?;
    }

    run_sql_script(&target_schema, &diff_target_tokio_config)?;

    let diff = run_migra(&config.diff_engine.source, &config.diff_engine.target)?;

    drop_db(&diff_source_tokio_config)?;
    drop_db(&diff_target_tokio_config)?;

    Ok(diff)
}

fn run_migra(source: &PostgresConfig, target: &PostgresConfig) -> Result<String> {
    let output = Command::new("migra")
        .arg(source.to_url())
        .arg(target.to_url())
        .arg("--unsafe")
        .output()?;
    if !output.stderr.is_empty() {
        bail!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn watch(args: &WatchArgs, config: &Config) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, tx)?;

    let path = Path::new(&args.path);
    let sql_extension = Some(OsStr::new("sql"));

    println!("watching {} ...", &args.path);
    debouncer.watcher().watch(path, RecursiveMode::Recursive)?;

    // just print all events, this blocks forever
    for e in rx.into_iter().flatten() {
        if e.iter()
            .any(|event| event.path.extension() == sql_extension)
        {
            println!("Deploying changes");
            let diff_source_tokio_config = config.diff_engine.source.to_tokio_postgres_config();
            drop_db(&diff_source_tokio_config)?;
            create_db(&diff_source_tokio_config)?;

            let mut source_schema = String::new();

            for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
                if entry.path().extension() == sql_extension {
                    source_schema += &fs::read_to_string(entry.path())?;
                }
            }

            run_sql_script(&source_schema, &diff_source_tokio_config)?;

            let diff = run_migra(&config.target, &config.diff_engine.source)?;

            let target_tokio_config = config.target.to_tokio_postgres_config();
            run_sql_script(&diff, &target_tokio_config)?;
        }
    }

    Ok(())
}

fn get_schema_script(repo_path: &str, ref_or_sha1: &str, schema_path: &str) -> Result<String> {
    let repo_path = Path::new(repo_path);
    let mut schema_path = Path::new(schema_path);
    if let Ok(p) = schema_path.strip_prefix("./") {
        schema_path = p
    }

    let repo = git_repository::open(repo_path)?;

    let oid = ObjectId::from_str(ref_or_sha1)?;
    let object_option = repo.try_find_object(oid)?;
    if let Some(object) = object_option {
        let commit = object.try_into_commit()?;
        let tree = commit.tree()?;

        let mut recorder = Recorder::default();

        tree.traverse().breadthfirst::<Recorder>(&mut recorder)?;

        let object_iter = recorder
            .records
            .iter()
            .filter(|entry| {
                matches!(entry.mode, EntryMode::Blob)
                    && entry.filepath.to_path().unwrap().starts_with(schema_path)
            })
            .map(|entry| repo.find_object(entry.oid))
            .filter_map(Result::ok);

        let mut script = String::new();
        for object in object_iter {
            script.push_str(object.data.to_str()?);
        }

        Ok(script)
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

    let query = format!(
        "drop database if exists {} (force)",
        config.get_dbname().unwrap()
    );
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
