use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let Some(expected) = &state.api_token else {
        return next.run(req).await;
    };

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(value) if value.starts_with("Bearer ") => {
            let token = &value["Bearer ".len()..];
            if token == expected {
                next.run(req).await
            } else {
                (StatusCode::UNAUTHORIZED, "invalid token").into_response()
            }
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            "missing or invalid authorization header",
        )
            .into_response(),
    }
}
