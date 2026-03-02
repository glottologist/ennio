use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::error::DbError;

/// Create a new PostgreSQL connection pool from a database URL.
pub async fn connect(database_url: &str) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Create a new PostgreSQL connection pool with custom max connections.
pub async fn connect_with_max(database_url: &str, max_connections: u32) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await?;
    Ok(pool)
}
