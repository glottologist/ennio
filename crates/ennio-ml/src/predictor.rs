use std::time::Duration;

use async_trait::async_trait;
use ennio_core::id::SessionId;

use crate::error::MlError;

/// Predicts session outcomes: success probability, duration, and cost.
#[async_trait]
pub trait SessionOutcomePredictor: Send + Sync {
    /// Returns the estimated probability (0.0..=1.0) that the session succeeds.
    async fn predict_success(&self, session_id: &SessionId) -> Result<f64, MlError>;

    async fn predict_duration(&self, session_id: &SessionId) -> Result<Duration, MlError>;

    async fn predict_cost(&self, session_id: &SessionId) -> Result<f64, MlError>;
}
