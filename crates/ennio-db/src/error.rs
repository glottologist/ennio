use ennio_core::error::EnnioError;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("row mapping error: {0}")]
    RowMapping(String),

    #[error("core error: {0}")]
    Core(#[from] EnnioError),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<DbError> for EnnioError {
    fn from(err: DbError) -> Self {
        EnnioError::Database {
            message: err.to_string(),
        }
    }
}
