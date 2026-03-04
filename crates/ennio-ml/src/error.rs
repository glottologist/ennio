/// Errors produced by ML operations.
#[derive(Debug, thiserror::Error)]
pub enum MlError {
    #[error("model not found: {name}")]
    ModelNotFound { name: String },

    #[error("prediction failed: {message}")]
    PredictionFailed { message: String },

    #[error("training failed: {message}")]
    TrainingFailed { message: String },

    #[error("insufficient data: required {required}, available {available}")]
    InsufficientData { required: usize, available: usize },

    #[error("internal ML error: {message}")]
    Internal { message: String },
}

impl MlError {
    pub fn model_not_found(name: impl Into<String>) -> Self {
        Self::ModelNotFound { name: name.into() }
    }

    pub fn prediction_failed(message: impl Into<String>) -> Self {
        Self::PredictionFailed {
            message: message.into(),
        }
    }

    pub fn training_failed(message: impl Into<String>) -> Self {
        Self::TrainingFailed {
            message: message.into(),
        }
    }

    pub fn insufficient_data(required: usize, available: usize) -> Self {
        Self::InsufficientData {
            required,
            available,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}
