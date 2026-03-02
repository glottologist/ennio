use rust_decimal::Decimal;

/// Errors specific to ledger operations.
#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("account not found: {id}")]
    AccountNotFound { id: String },

    #[error("budget not found for project {project_id}")]
    BudgetNotFound { project_id: String },

    #[error("budget exceeded: limit {limit}, used {used}, requested {requested}")]
    BudgetExceeded {
        limit: Decimal,
        used: Decimal,
        requested: Decimal,
    },

    #[error("invalid amount: {reason}")]
    InvalidAmount { reason: String },

    #[error("transfer failed: {reason}")]
    TransferFailed { reason: String },

    #[error("duplicate entry: {entity} with id {id}")]
    Duplicate { entity: String, id: String },

    #[error("internal ledger error: {message}")]
    Internal { message: String },
}

impl From<LedgerError> for ennio_core::error::EnnioError {
    fn from(err: LedgerError) -> Self {
        Self::Ledger {
            message: err.to_string(),
        }
    }
}
