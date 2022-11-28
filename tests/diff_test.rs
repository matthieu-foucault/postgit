#![feature(thread_id_value)]

use postgit::{self, DiffArgs};

mod common;
pub use common::*;

#[test]
fn it_returns_diff_string() {
    let repo = setup();
    let config = get_config();
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
    let config = get_config();
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
    let config = get_config();
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
    let config = get_config();
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

#[test]
fn it_handles_revision_specs() {
    let repo = setup();
    let config = get_config();
    let args = DiffArgs {
        from: Some("HEAD^1".to_string()),
        to: "HEAD".to_string(),
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

#[test]
fn it_handles_custom_commands() {
    let repo = setup();
    let mut config = get_config();
    config.diff_engine.command = Some(r#"echo "$1 - $2""#.to_string());
    let args = DiffArgs {
        from: Some(repo.commits[0].to_owned()),
        to: repo.commits[1].to_owned(),
        path: String::from("schema.sql"),
        repo_path: repo.repo_path,
        source_path: None,
    };

    let diff_string = postgit::get_diff_string(&args, &config).unwrap();
    assert_eq!(
        format!(
            "{} - {}",
            config.diff_engine.source.to_url(),
            config.diff_engine.target.to_url()
        ),
        diff_string
    );
}
