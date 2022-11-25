use anyhow::Result;

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::ffi::OsStr;
use std::fs::{self};
use std::path::Path;
use std::time::Duration;
use walkdir::WalkDir;

pub mod cli;
pub use cli::*;

pub mod config;
use config::*;

mod diff;

pub mod db;
use db::*;

mod repo;
use repo::get_schema_script;

pub fn apply_diff(args: &DiffArgs, config: &Config) -> Result<()> {
    let diff_string = get_diff_string(args, config)?;
    let target_tokio_config = config.target.to_tokio_postgres_config();
    db::run_sql_script(&diff_string, &target_tokio_config)
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

    let diff = diff::run_migra(&config.diff_engine.source, &config.diff_engine.target)?;

    drop_db(&diff_source_tokio_config)?;
    drop_db(&diff_target_tokio_config)?;

    Ok(diff)
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

            let diff_string = diff::run_migra(&config.target, &config.diff_engine.source)?;

            let target_tokio_config = config.target.to_tokio_postgres_config();
            run_sql_script(&diff_string, &target_tokio_config)?;
        }
    }

    Ok(())
}
