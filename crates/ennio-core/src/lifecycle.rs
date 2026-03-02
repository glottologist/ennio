use std::collections::HashMap;

use async_trait::async_trait;

use crate::error::EnnioError;
use crate::id::{ProjectId, SessionId};
use crate::session::{Session, SessionStatus};

#[derive(Debug, Clone)]
pub struct SpawnRequest<'a> {
    pub project_id: &'a ProjectId,
    pub issue_id: Option<&'a str>,
    pub prompt: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub role: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub sessions_cleaned: u32,
    pub sessions_failed: u32,
    pub details: Vec<CleanupDetail>,
}

#[derive(Debug, Clone)]
pub struct CleanupDetail {
    pub session_id: SessionId,
    pub success: bool,
    pub reason: String,
}

#[async_trait]
pub trait SessionManager: Send + Sync {
    async fn spawn(&self, request: &SpawnRequest<'_>) -> Result<Session, EnnioError>;

    async fn restore(&self, session_id: &SessionId) -> Result<Session, EnnioError>;

    async fn list(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>, EnnioError>;

    async fn get(&self, session_id: &SessionId) -> Result<Session, EnnioError>;

    async fn kill(&self, session_id: &SessionId) -> Result<(), EnnioError>;

    async fn cleanup(&self, project_id: &ProjectId) -> Result<CleanupResult, EnnioError>;

    async fn send(&self, session_id: &SessionId, message: &str) -> Result<(), EnnioError>;
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: SessionId,
    pub status: SessionStatus,
    pub last_checked: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait LifecycleManager: Send + Sync {
    async fn start(&self) -> Result<(), EnnioError>;

    async fn stop(&self) -> Result<(), EnnioError>;

    async fn get_states(&self) -> Result<HashMap<SessionId, SessionState>, EnnioError>;

    async fn check(&self, session_id: &SessionId) -> Result<SessionState, EnnioError>;
}
