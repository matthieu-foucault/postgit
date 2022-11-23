use postgit::{self, Config, DiffArgs};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

struct Repo {
    repo_path: String,
    commits: Vec<String>,
}

fn commit_all(repo_path: &str) -> String {
    Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("some message")
        .current_dir(repo_path)
        .output()
        .unwrap();

    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()
        .unwrap();

    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn setup() -> Repo {
    let repo_path = tempdir().unwrap().into_path();
    let mut commits: Vec<String> = vec![];
    let mut schema_file_path = repo_path.clone();
    schema_file_path.push("schema.sql");

    Command::new("git")
        .arg("init")
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

#[test]
fn it_returns_diff_string() {
    let repo = setup();
    let config = Config::build().unwrap();
    let args = DiffArgs {
        from: Some(repo.commits[0].to_owned()),
        to: repo.commits[1].to_owned(),
        path: String::from("schema.sql"),
        repo_path: repo.repo_path,
        source_path: None,
    };

    let diff_string = postgit::get_diff_string(&args, &config).unwrap();
    assert_eq!(
        r#"alter table "my_app"."user" alter column "email" set not null;"#,
        diff_string
    );
}

#[test]
fn it_handles_relative_path() {
    let repo = setup();
    let config = Config::build().unwrap();
    let args = DiffArgs {
        from: Some(repo.commits[0].to_owned()),
        to: repo.commits[1].to_owned(),
        path: String::from("./schema.sql"),
        repo_path: repo.repo_path,
        source_path: None,
    };

    let diff_string = postgit::get_diff_string(&args, &config).unwrap();
    assert_eq!(
        r#"alter table "my_app"."user" alter column "email" set not null;"#,
        diff_string
    );
}

#[test]
fn it_handles_directories() {
    let repo = setup();
    let config = Config::build().unwrap();
    let args = DiffArgs {
        from: Some(repo.commits[0].to_owned()),
        to: repo.commits[1].to_owned(),
        path: String::from("./"),
        repo_path: repo.repo_path,
        source_path: None,
    };

    let diff_string = postgit::get_diff_string(&args, &config).unwrap();
    assert_eq!(
        r#"alter table "my_app"."user" alter column "email" set not null;"#,
        diff_string
    );
}

#[test]
fn it_handles_multiple_files() {
    let repo = setup();
    let config = Config::build().unwrap();
    let args = DiffArgs {
        from: Some(repo.commits[1].to_owned()),
        to: repo.commits[2].to_owned(),
        path: String::from("./schema/"),
        repo_path: repo.repo_path,
        source_path: Some(String::from("./")),
    };

    let diff_string = postgit::get_diff_string(&args, &config).unwrap();
    assert_eq!(
        r#"alter table "my_app"."user" alter column "given_name" set not null;"#,
        diff_string
    );
}
