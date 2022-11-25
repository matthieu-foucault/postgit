#![feature(thread_id_value)]

mod common;
use common::*;
use postgit::DiffArgs;
use tokio_postgres::{NoTls, Row};

#[tokio::main]
async fn execute_statement(config: &tokio_postgres::Config, statement: &str) -> Vec<Row> {
    let (client, connection) = config.connect(NoTls).await.unwrap();
    tokio::spawn(connection);

    let rows = client.query(statement, &[]).await.unwrap();
    rows
}

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
    postgit::db::create_db(&target_config).unwrap();

    postgit::apply_diff(&args, &config).unwrap();

    let rows = execute_statement(&target_config,
            "select column_name from information_schema.columns where table_schema = 'my_app' and table_name = 'user';"
        );

    assert_eq!(4, rows.len());
}
