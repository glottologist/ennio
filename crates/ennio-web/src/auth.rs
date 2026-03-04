use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::state::AppState;

fn tokens_equal(a: &str, b: &str) -> bool {
    let hash_a = Sha256::digest(a.as_bytes());
    let hash_b = Sha256::digest(b.as_bytes());
    hash_a.ct_eq(&hash_b).into()
}

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Response {
    let Some(expected) = &state.api_token else {
        return (StatusCode::UNAUTHORIZED, "no API token configured").into_response();
    };

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(value) if value.starts_with("Bearer ") => {
            let token = &value["Bearer ".len()..];
            if tokens_equal(token, expected) {
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
