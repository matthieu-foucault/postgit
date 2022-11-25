use git_repository::bstr::ByteSlice;
use postgit::config::*;
use std::fs;
use std::process::Command;
use std::thread;
use tempfile::tempdir;

#[derive(Debug)]
pub struct Repo {
    pub repo_path: String,
    pub commits: Vec<String>,
}

pub fn get_config() -> Config {
    let thread_id = thread::current().id().as_u64();
    let config: Config = toml::from_str(
        format!(
            "
[diff_engine]

[diff_engine.source]
dbname='postgres_vcs_source_{thread_id}'
host='localhost'
port=5432
user='postgres'

[diff_engine.target]
dbname='postgres_vcs_target_{thread_id}'
host='localhost'
port=5432
user='postgres'

[target]
dbname='postgit_test_{thread_id}'
host='localhost'
port=5432
user='postgres'
    ",
            thread_id = thread_id
        )
        .as_str(),
    )
    .unwrap();
    config
}

pub fn commit_all(repo_path: &str) -> String {
    let add_out = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(repo_path)
        .output()
        .unwrap();

    println!("{}", add_out.stdout.to_str().unwrap());
    println!("{}", add_out.stderr.to_str().unwrap());

    let commit_out = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("some message")
        .current_dir(repo_path)
        .output()
        .unwrap();

    println!("{}", commit_out.stdout.to_str().unwrap());
    println!("{}", commit_out.stderr.to_str().unwrap());

    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()
        .unwrap();

    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub fn setup() -> Repo {
    let repo_path = tempdir().unwrap().into_path();
    let mut commits: Vec<String> = vec![];
    let mut schema_file_path = repo_path.clone();
    schema_file_path.push("schema.sql");

    Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("test")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    fs::write(
        &schema_file_path,
        r#"create schema my_app;
        create table my_app.user (
          id int primary key generated always as identity,
          given_name text,
          family_name text,
          email text
        );"#,
    )
    .unwrap();

    commits.push(commit_all(repo_path.to_str().unwrap()));

    // add not null constraint to email
    fs::write(
        &schema_file_path,
        r#"create schema my_app;
          create table my_app.user (
            id int primary key generated always as identity,
            given_name text,
            family_name text,
            email text not null
          );"#,
    )
    .unwrap();

    commits.push(commit_all(repo_path.to_str().unwrap()));

    fs::remove_file(&schema_file_path).unwrap();

    fs::create_dir(repo_path.join("schema")).unwrap();

    fs::write(
        repo_path.join("schema/001_schema.sql"),
        "create schema my_app;",
    )
    .unwrap();

    fs::write(
        repo_path.join("schema/002_user.sql"),
        r#"
          create table my_app.user (
            id int primary key generated always as identity,
            given_name text not null,
            family_name text,
            email text not null
          );"#,
    )
    .unwrap();

    commits.push(commit_all(repo_path.to_str().unwrap()));

    Repo {
        repo_path: repo_path.to_str().unwrap().to_string(),
        commits,
    }
}
