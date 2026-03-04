use async_trait::async_trait;
use ennio_core::id::SessionId;

use crate::error::MlError;

/// Detects anomalous metric values for a session.
#[async_trait]
pub trait AnomalyDetector: Send + Sync {
    async fn is_anomalous(
        &self,
        session_id: &SessionId,
        metric: &str,
        value: f64,
    ) -> Result<bool, MlError>;

    /// Returns a score (0.0..=1.0) indicating how anomalous the value is.
    async fn get_anomaly_score(
        &self,
        session_id: &SessionId,
        metric: &str,
        value: f64,
    ) -> Result<f64, MlError>;
}
