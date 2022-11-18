use clap::Parser;
use std::error::Error;
use std::io::prelude::*;
use std::process::Command;
use std::{fs::File, path::Path};
use tokio_postgres::NoTls;

/// Migrate from source to target postgres schemas
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the root of the git repository
    #[arg(long, short)]
    pub repo_path: String,

    // /// Git ref where the source schema can be found
    // #[arg(long, short)]
    // pub source_ref: String,

    // /// Git ref where the target schema can be found
    // #[arg(long, short)]
    // pub target_ref: String,
    /// Path to the source schema at the source ref
    #[arg(long)]
    pub source_path: String,

    /// Path to the target schema at the target ref
    #[arg(long)]
    pub target_path: String,
}

pub fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    let source_path = Path::new(&args.repo_path).join(&args.source_path);
    let target_path = Path::new(&args.repo_path).join(&args.target_path);

    let source_db = String::from("postgres_vcs_source");
    create_db(&source_db)?;
    let target_db = String::from("postgres_vcs_target");
    create_db(&target_db)?;
    run_sql_script(&source_path, &source_db)?;
    run_sql_script(&target_path, &target_db)?;

    let output = Command::new("migra")
        .arg("postgresql:///postgres_vcs_source")
        .arg("postgresql:///postgres_vcs_target")
        .output()?;

    drop_db(&source_db)?;
    drop_db(&target_db)?;

    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    println!("{}", String::from_utf8_lossy(&output.stdout).trim());

    Ok(())
}

#[tokio::main]
async fn create_db(db_name: &String) -> Result<(), Box<dyn Error>> {
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=postgres", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!("create database {}", db_name);
    client.batch_execute(&query).await?;

    Ok(())
}

#[tokio::main]
async fn drop_db(db_name: &String) -> Result<(), Box<dyn Error>> {
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=postgres", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!("drop database {}", db_name);
    client.batch_execute(&query).await?;

    Ok(())
}

#[tokio::main]
async fn run_sql_script(path: &Path, db_name: &String) -> Result<(), Box<dyn Error>> {
    let connection_str = format!("host=localhost user=postgres dbname={}", db_name);
    // Connect to the database.
    let (client, connection) = tokio_postgres::connect(&connection_str, NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = File::open(path)?;

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut s = String::new();
    file.read_to_string(&mut s)?;

    // Now we can execute a simple statement that just returns its parameter.
    client.batch_execute(&s).await?;

    Ok(())
}
