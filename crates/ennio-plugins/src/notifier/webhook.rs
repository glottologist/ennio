use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::event::OrchestratorEvent;
use ennio_core::notifier::{Notifier, NotifyAction, NotifyContext};
use reqwest::Client;
use tracing::debug;

pub struct WebhookNotifier {
    client: Client,
    url: String,
}

impl WebhookNotifier {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            url: url.into(),
        }
    }
}

#[async_trait]
impl Notifier for WebhookNotifier {
    fn name(&self) -> &str {
        "webhook"
    }

    async fn notify(&self, event: &OrchestratorEvent) -> Result<(), EnnioError> {
        debug!(
            event_type = %event.event_type,
            session_id = %event.session_id,
            "sending webhook notification"
        );

        let payload = serde_json::json!({
            "event_type": event.event_type.to_string(),
            "priority": event.priority.to_string(),
            "session_id": event.session_id.to_string(),
            "project_id": event.project_id.to_string(),
            "message": event.message,
            "timestamp": event.timestamp.to_rfc3339(),
        });

        self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("webhook request failed: {e}"),
            })?;

        Ok(())
    }

    async fn notify_with_actions(
        &self,
        event: &OrchestratorEvent,
        actions: &[NotifyAction],
    ) -> Result<(), EnnioError> {
        let payload = serde_json::json!({
            "event_type": event.event_type.to_string(),
            "priority": event.priority.to_string(),
            "session_id": event.session_id.to_string(),
            "project_id": event.project_id.to_string(),
            "message": event.message,
            "timestamp": event.timestamp.to_rfc3339(),
            "actions": actions.iter().map(|a| serde_json::json!({
                "label": a.label,
                "action": a.action,
                "url": a.url,
            })).collect::<Vec<_>>(),
        });

        self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("webhook request failed: {e}"),
            })?;

        Ok(())
    }

    async fn post(&self, context: &NotifyContext, message: &str) -> Result<(), EnnioError> {
        let payload = serde_json::json!({
            "session_id": context.session_id.to_string(),
            "project_name": context.project_name,
            "priority": context.priority.to_string(),
            "summary": context.summary,
            "message": message,
        });

        self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("webhook request failed: {e}"),
            })?;

        Ok(())
    }
}
