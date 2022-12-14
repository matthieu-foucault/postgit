mod common;
pub use common::*;
use postgit::{config::DiffEngineConfig, deploy_changes, WatchArgs};
use std::{fs, thread, time::Duration};
use tempfile::tempdir;

#[test]
#[ignore = "skipping for now"]
fn it_watches_file_writes() {
    let config = get_config();
    let target_config = config.target.to_tokio_postgres_config();
    let dir = tempdir().unwrap().into_path();
    let args = WatchArgs {
        path: dir.display().to_string(),
    };

    thread::spawn(move || postgit::watch(&args, &config));

    let mut schema_file_path = dir.clone();
    schema_file_path.push("001_schema.sql");

    let mut table_file_path = dir;
    table_file_path.push("002_table.sql");

    fs::write(schema_file_path, r#"create schema my_app;"#).unwrap();

    fs::write(
        table_file_path,
        r#"create table my_app.user (
          id int primary key generated always as identity,
          given_name text,
          family_name text,
          email text
        );"#,
    )
    .unwrap();

    thread::sleep(Duration::from_secs(4));

    let rows = execute_statement(&target_config,
        "select column_name from information_schema.columns where table_schema = 'my_app' and table_name = 'user';"
    );

    assert_eq!(4, rows.len());
}

#[test]
#[ignore = "skipping for now"]
fn it_handles_errors_in_sql_files() {
    let config = get_config();
    let target_config = config.target.to_tokio_postgres_config();
    let dir = tempdir().unwrap().into_path();
    let args = WatchArgs {
        path: dir.display().to_string(),
    };

    thread::spawn(move || postgit::watch(&args, &config));

    let mut table_file_path = dir;
    table_file_path.push("001_table.sql");

    fs::write(
        &table_file_path,
        r#"create table public.user (
          id int primary key generated always as identity,
          given_name text,
          family_name text,
          email text,
        );"#,
    )
    .unwrap();
    // first write has an extra semicolon after the last column

    thread::sleep(Duration::from_secs(4));

    fs::write(
        &table_file_path,
        r#"create table public.user (
          id int primary key generated always as identity,
          given_name text,
          family_name text,
          email text
        );"#,
    )
    .unwrap();

    thread::sleep(Duration::from_secs(4));

    let rows = execute_statement(&target_config,
        "select column_name from information_schema.columns where table_schema = 'public' and table_name = 'user';"
    );

    assert_eq!(4, rows.len());
}

#[test]
fn it_handles_file_imports() {
    let config = get_config();
    let target_config = config.target.to_tokio_postgres_config();
    let dir = tempdir().unwrap().into_path();

    let mut user_file_path = dir.clone();
    user_file_path.push("user.sql");

    let mut todo_file_path = dir.clone();
    todo_file_path.push("todo.sql");

    fs::write(
        user_file_path,
        r#"create table public.user (
        id int primary key generated always as identity,
        given_name text,
        family_name text,
        email text
      );"#,
    )
    .unwrap();

    fs::write(
        todo_file_path,
        r#"
        -- import ./user.sql
        create table public.todo (
          id int primary key generated always as identity,
          task text,
          created_by int references public.user(id) not null
        );"#,
    )
    .unwrap();

    let watch_config = DiffEngineConfig {
        command: config.diff_engine.command.clone(),
        source: config.target.clone(),
        target: config.diff_engine.source.clone(),
    };

    deploy_changes(&config, dir.as_path(), &watch_config).unwrap();

    let user_cols = execute_statement(
        &target_config,
        "select column_name from information_schema.columns where table_name = 'user';",
    );

    assert_eq!(4, user_cols.len());

    let todo_cols = execute_statement(
        &target_config,
        "select column_name from information_schema.columns where table_name = 'todo';",
    );

    assert_eq!(3, todo_cols.len());
}
