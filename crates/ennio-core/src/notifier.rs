use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::EnnioError;
use crate::event::{EventPriority, OrchestratorEvent};
use crate::id::SessionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyAction {
    pub label: String,
    pub action: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyContext {
    pub session_id: SessionId,
    pub project_name: String,
    pub priority: EventPriority,
    pub summary: Option<String>,
}

#[async_trait]
pub trait Notifier: Send + Sync {
    fn name(&self) -> &str;

    async fn notify(&self, event: &OrchestratorEvent) -> Result<(), EnnioError>;

    async fn notify_with_actions(
        &self,
        event: &OrchestratorEvent,
        actions: &[NotifyAction],
    ) -> Result<(), EnnioError>;

    async fn post(&self, context: &NotifyContext, message: &str) -> Result<(), EnnioError>;
}
