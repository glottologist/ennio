use async_trait::async_trait;
use ennio_core::id::SessionId;

use crate::error::MlError;

/// Predicts session costs based on historical data and issue complexity.
#[async_trait]
pub trait CostPredictor: Send + Sync {
    /// Estimates the remaining cost for an active session.
    async fn estimate_remaining_cost(&self, session_id: &SessionId) -> Result<f64, MlError>;

    /// Estimates the total cost for a new issue given its complexity score.
    async fn estimate_total_cost(&self, issue_complexity: f64) -> Result<f64, MlError>;
}
