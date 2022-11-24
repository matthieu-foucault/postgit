use anyhow::Result;
use tokio_postgres::NoTls;

#[tokio::main]
pub async fn create_db(config: &tokio_postgres::Config) -> Result<()> {
    let mut parent_config = config.clone();
    parent_config.dbname("postgres");

    let (client, connection) = parent_config.connect(NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!("create database {}", config.get_dbname().unwrap());
    client.batch_execute(&query).await?;

    Ok(())
}

#[tokio::main]
pub async fn drop_db(config: &tokio_postgres::Config) -> Result<()> {
    let mut parent_config = config.clone();
    parent_config.dbname("postgres");

    let (client, connection) = parent_config.connect(NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = format!(
        "drop database if exists {} (force)",
        config.get_dbname().unwrap()
    );
    client.batch_execute(&query).await?;

    Ok(())
}

#[tokio::main]
pub async fn run_sql_script(script: &str, config: &tokio_postgres::Config) -> Result<()> {
    // Connect to the database.
    let (mut client, connection) = config.connect(NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Now we can execute a simple statement that just returns its parameter.
    let transaction = client.transaction().await?;
    transaction.batch_execute(script).await?;
    transaction.commit().await?;

    Ok(())
}
