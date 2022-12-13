mod common;
pub use common::*;
use postgit::DiffArgs;

#[test]
fn it_pushes_initial_commit() {
    let repo = setup();
    let config = get_config();
    let args = DiffArgs {
        from: None,
        to: repo.commits[1].to_owned(),
        path: String::from("schema.sql"),
        repo_path: repo.repo_path,
        source_path: None,
    };
    let target_config = config.target.to_tokio_postgres_config();
    postgit::db::drop_db(&target_config).unwrap();
    postgit::db::create_db(&target_config).unwrap();

    postgit::apply_diff(&args, &config).unwrap();

    let rows = execute_statement(&target_config,
            "select column_name from information_schema.columns where table_schema = 'my_app' and table_name = 'user';"
        );

    assert_eq!(4, rows.len());
}

#[test]
fn it_pushes_subsequent_commits() {
    let repo = setup();
    let config = get_config();
    let args = DiffArgs {
        from: None,
        to: repo.commits[0].to_owned(),
        path: String::from("schema.sql"),
        repo_path: repo.repo_path.to_owned(),
        source_path: None,
    };
    let target_config = config.target.to_tokio_postgres_config();
    postgit::db::drop_db(&target_config).unwrap();
    postgit::db::create_db(&target_config).unwrap();

    postgit::apply_diff(&args, &config).unwrap();

    let args = DiffArgs {
        from: Some(repo.commits[0].to_owned()),
        to: repo.commits[1].to_owned(),
        path: String::from("schema.sql"),
        repo_path: repo.repo_path,
        source_path: None,
    };

    postgit::apply_diff(&args, &config).unwrap();

    let rows = execute_statement(&target_config,
            "select column_name from information_schema.columns where table_schema = 'my_app' and table_name = 'user';"
        );

    assert_eq!(4, rows.len());
}
