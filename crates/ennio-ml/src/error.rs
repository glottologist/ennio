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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(MlError::model_not_found("gpt-4"), "model not found: gpt-4")]
    #[case(MlError::prediction_failed("timeout"), "prediction failed: timeout")]
    #[case(MlError::training_failed("no data"), "training failed: no data")]
    #[case(
        MlError::insufficient_data(100, 5),
        "insufficient data: required 100, available 5"
    )]
    #[case(MlError::internal("oops"), "internal ML error: oops")]
    fn error_display(#[case] error: MlError, #[case] expected: &str) {
        assert_eq!(error.to_string(), expected);
    }

    #[rstest]
    #[case(MlError::model_not_found("x"), "ModelNotFound")]
    #[case(MlError::prediction_failed("x"), "PredictionFailed")]
    #[case(MlError::training_failed("x"), "TrainingFailed")]
    #[case(MlError::insufficient_data(1, 0), "InsufficientData")]
    #[case(MlError::internal("x"), "Internal")]
    fn error_variant_debug(#[case] error: MlError, #[case] expected_variant: &str) {
        let debug = format!("{error:?}");
        assert!(debug.contains(expected_variant));
    }
}
