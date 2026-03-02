use std::sync::Arc;

use ennio_core::lifecycle::{LifecycleManager, SessionManager};

pub struct AppState {
    pub session_manager: Arc<dyn SessionManager>,
    pub lifecycle_manager: Arc<dyn LifecycleManager>,
    pub api_token: Option<String>,
    pub cors_origins: Vec<String>,
}
