use std::sync::{Arc, LazyLock};

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use axum::middleware;
use axum::routing::{delete, get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};
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

    let parsed: Vec<_> = origins.iter().filter_map(|o| o.parse().ok()).collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(parsed))
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
}
