use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

use crate::error::DbError;

fn connect_options(database_url: &str) -> Result<SqliteConnectOptions, DbError> {
    let options: SqliteConnectOptions = database_url
        .parse()
        .map_err(|e: sqlx::Error| DbError::Connection(e.to_string()))?;

    Ok(options
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true))
}

pub async fn connect(database_url: &str) -> Result<SqlitePool, DbError> {
    let options = connect_options(database_url)?;
    let pool = SqlitePool::connect_with(options).await?;
    Ok(pool)
}

pub async fn connect_with_max(
    database_url: &str,
    max_connections: u32,
) -> Result<SqlitePool, DbError> {
    let options = connect_options(database_url)?;
    let pool = sqlx::pool::PoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;
    Ok(pool)
}
