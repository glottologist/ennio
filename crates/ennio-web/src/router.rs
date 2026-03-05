use std::sync::{Arc, LazyLock};

use axum::Router;
use axum::http::{HeaderName, HeaderValue, Method, header};
use axum::middleware;
use axum::routing::{delete, get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::auth;
use crate::handlers;
use crate::state::AppState;

static DEFAULT_ORIGINS: LazyLock<[HeaderValue; 2]> = LazyLock::new(|| {
    [
        "http://localhost:3000".parse().unwrap(),
        "http://localhost:3001".parse().unwrap(),
    ]
});

const MAX_REQUEST_BODY_BYTES: usize = 1_048_576;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = build_cors(&state.cors_origins);

    let authenticated = Router::new()
        .route("/sessions", get(handlers::list_sessions))
        .route("/sessions/{id}", get(handlers::get_session))
        .route("/sessions", post(handlers::spawn_session))
        .route("/sessions/{id}", delete(handlers::kill_session))
        .route("/sessions/{id}/send", post(handlers::send_to_session))
        .route_layer(middleware::from_fn_with_state(
            state.clone(), // clone: Arc clone for middleware state
            auth::require_auth,
        ));

    let api = Router::new()
        .route("/health", get(handlers::health))
        .merge(authenticated);

    Router::new()
        .nest("/api/v1", api)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("0"),
        ))
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        .with_state(state)
}

fn build_cors(origins: &[String]) -> CorsLayer {
    let allowed_methods = [Method::GET, Method::POST, Method::DELETE];
    let allowed_headers = [header::CONTENT_TYPE, header::AUTHORIZATION];

    if origins.is_empty() {
        return CorsLayer::new()
            .allow_origin(AllowOrigin::list(DEFAULT_ORIGINS.clone()))
            .allow_methods(allowed_methods)
            .allow_headers(allowed_headers);
    }

    let parsed: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|o| match o.parse() {
            Ok(val) => Some(val),
            Err(err) => {
                tracing::warn!(origin = %o, error = %err, "skipping unparseable CORS origin");
                None
            }
        })
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(parsed))
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
}
