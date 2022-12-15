use anyhow::Result;

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self};
use std::io::{self, Write};
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

use crate::repo::merge_sql_scripts;

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

    let diff = diff::run_diff_command(&config.diff_engine)?;

    drop_db(&diff_source_tokio_config)?;
    drop_db(&diff_target_tokio_config)?;

    Ok(diff)
}

pub fn deploy_changes(config: &Config, path: &Path, watch_config: &DiffEngineConfig) -> Result<()> {
    let sql_extension = Some(OsStr::new("sql"));

    print!("deploying changes ");
    io::stdout().flush()?;
    let diff_source_tokio_config = config.diff_engine.source.to_tokio_postgres_config();
    drop_db(&diff_source_tokio_config)?;
    create_db(&diff_source_tokio_config)?;

    let file_entries = WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension() == sql_extension)
        .map(|e| (e.path().to_owned(), fs::read_to_string(e.path()).unwrap()))
        .collect::<Vec<_>>();

    let sql_scripts = file_entries
        .iter()
        .map(|e| (e.0.to_str().unwrap(), e.1.as_str()))
        .collect::<HashMap<&str, &str>>();

    let source_schema = merge_sql_scripts(&sql_scripts)?;

    let source_deploy_result = run_sql_script(&source_schema, &diff_source_tokio_config);

    match source_deploy_result {
        Err(err) => {
            println!("❌");
            eprintln!("The schema in the watched directory could not be deployed.");
            eprintln!("{}", err);
            Ok(())
        }
        Ok(_) => {
            let mut diff_string = diff::run_diff_command(watch_config)?;

            let target_tokio_config = config.target.to_tokio_postgres_config();
            let apply_diff_result = run_sql_script(&diff_string, &target_tokio_config);
            if let Err(err) = apply_diff_result {
                println!("❌");
                eprintln!("Could not apply the changes to the target db.\n{}", err);
                if config.watch.recreate_db_on_fail {
                    println!("Recreating target db");
                    drop_db(&target_tokio_config)?;
                    create_db(&target_tokio_config)?;
                    diff_string = diff::run_diff_command(watch_config)?;
                    run_sql_script(&diff_string, &target_tokio_config).unwrap_or_else(|err| {
                        eprintln!("Failed again, retrying on the next file change.\n{}", err);
                    });
                }
            } else {
                println!("✓");
            }
            Ok(())
        }
    }
}

pub fn watch(args: &WatchArgs, config: &Config) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, tx)?;

    let path = Path::new(&args.path);
    let sql_extension = Some(OsStr::new("sql"));

    println!("watching {} ...", &args.path);
    debouncer.watcher().watch(path, RecursiveMode::Recursive)?;

    let watch_config = DiffEngineConfig {
        command: config.diff_engine.command.clone(),
        source: config.target.clone(),
        target: config.diff_engine.source.clone(),
    };

    // just print all events, this blocks forever
    for e in rx.into_iter().flatten() {
        if e.iter()
            .any(|event| event.path.extension() == sql_extension)
        {
            deploy_changes(config, path, &watch_config)?;
        }
    }

    Ok(())
}
