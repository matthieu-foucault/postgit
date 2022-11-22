use postgit::{self, Config, DiffArgs};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

struct Repo {
    repo_path: String,
    commits: Vec<String>,
}

fn setup() -> Repo {
    let repo_path = tempdir().unwrap().into_path();
    let repo_path_string = repo_path.to_str().unwrap().to_owned();
    let mut schema_file_path = repo_path.clone();
    schema_file_path.push("schema.sql");

    Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()
        .expect("failed to execute process");

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
    .expect("couldn't write to file");

    Command::new("git")
        .arg("add")
        .arg("schema.sql")
        .current_dir(&repo_path)
        .output()
        .expect("failed to execute process");

    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("first commit")
        .current_dir(&repo_path)
        .output()
        .expect("failed to execute process");

    let first_commit = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
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
            email text not null
          );"#,
    )
    .expect("couldn't write to file");

    Command::new("git")
        .arg("commit")
        .arg("-am")
        .arg("second commit")
        .current_dir(&repo_path)
        .output()
        .expect("failed to execute process");

    let second_commit = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(&repo_path)
        .output()
        .unwrap();

    let commits: Vec<String> = vec![
        String::from_utf8_lossy(&first_commit.stdout)
            .trim()
            .to_string(),
        String::from_utf8_lossy(&second_commit.stdout)
            .trim()
            .to_string(),
    ];

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
        from: repo.commits[0].to_owned(),
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
        from: repo.commits[0].to_owned(),
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
        from: repo.commits[0].to_owned(),
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
