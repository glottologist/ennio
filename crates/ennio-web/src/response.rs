use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use ennio_core::error::EnnioError;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: u16,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        match serde_json::to_string(&self) {
            Ok(body) => {
                (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
            }
            Err(e) => {
                tracing::error!("ApiError serialization failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

impl From<EnnioError> for ApiError {
    fn from(err: EnnioError) -> Self {
        let (code, message) = match &err {
            EnnioError::NotFound { .. } => (404, err.to_string()),
            EnnioError::AlreadyExists { .. } => (409, err.to_string()),
            EnnioError::InvalidId { .. } => (400, err.to_string()),
            EnnioError::Config { .. } => (400, err.to_string()),
            EnnioError::Budget { .. } => (402, err.to_string()),
            EnnioError::Timeout { .. } => (504, err.to_string()),
            _ => {
                tracing::error!(error = %err, "internal server error");
                (500, "internal server error".to_owned())
            }
        };
        Self {
            error: message,
            code,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        match serde_json::to_string(&self) {
            Ok(body) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                body,
            )
                .into_response(),
            Err(e) => {
                tracing::error!("ApiResponse serialization failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use ennio_core::error::EnnioError;
    use ennio_core::id::ProjectId;
    use rstest::rstest;

    #[rstest]
    #[case(
        EnnioError::NotFound { entity: "session".to_owned(), id: "s1".to_owned() },
        404
    )]
    #[case(
        EnnioError::AlreadyExists { entity: "project".to_owned(), id: "p1".to_owned() },
        409
    )]
    #[case(
        EnnioError::InvalidId { value: "bad".to_owned(), reason: "too short".to_owned() },
        400
    )]
    #[case(
        EnnioError::Config { message: "bad config".to_owned() },
        400
    )]
    #[case(
        EnnioError::Timeout { duration: std::time::Duration::from_secs(30), message: "timed out".to_owned() },
        504
    )]
    #[case(
        EnnioError::Internal { message: "unexpected".to_owned() },
        500
    )]
    #[case(
        EnnioError::Database { message: "connection lost".to_owned() },
        500
    )]
    #[case(
        EnnioError::Ssh { message: "auth failed".to_owned() },
        500
    )]
    #[case(
        EnnioError::Nats { message: "disconnected".to_owned() },
        500
    )]
    #[case(
        EnnioError::Tracker { message: "api error".to_owned() },
        500
    )]
    #[case(
        EnnioError::Scm { message: "api error".to_owned() },
        500
    )]
    fn api_error_from_ennio_error_status_code(#[case] err: EnnioError, #[case] expected_code: u16) {
        let api_err = ApiError::from(err);
        assert_eq!(api_err.code, expected_code);
    }

    #[test]
    fn api_error_from_budget_error_is_402() {
        let project_id = ProjectId::new("p1").unwrap();
        let err = EnnioError::Budget {
            project_id,
            message: "over limit".to_owned(),
        };
        let api_err = ApiError::from(err);
        assert_eq!(api_err.code, 402);
    }

    #[test]
    fn api_error_into_response_has_json_content_type() {
        let err = ApiError {
            error: "not found".to_owned(),
            code: 404,
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn api_response_into_response_has_json_content_type() {
        let resp = ApiResponse {
            data: serde_json::json!({"key": "value"}),
        };
        let response = resp.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn api_error_response_body_is_valid_json() {
        let err = ApiError {
            error: "test error".to_owned(),
            code: 500,
        };
        let json = serde_json::to_string(&err).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["error"], "test error");
        assert_eq!(parsed["code"], 500);
    }
}
