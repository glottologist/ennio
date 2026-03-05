use ennio_core::event::OrchestratorEvent;
use serde::Serialize;

use crate::client::NatsClient;
use crate::error::NatsError;
use crate::topics;

pub struct EventPublisher {
    client: Option<NatsClient>,
}

impl EventPublisher {
    pub fn new(client: NatsClient) -> Self {
        Self {
            client: Some(client),
        }
    }

    pub fn without_nats() -> Self {
        Self { client: None }
    }

    pub async fn publish_event(&self, event: &OrchestratorEvent) -> Result<(), NatsError> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()),
        };
        let topic = topics::topic_for_event_type(event.event_type, event.project_id.as_str())?;
        client.publish_json(&topic, event).await
    }

    pub async fn publish_session_event(
        &self,
        project_id: &str,
        action: &str,
        data: &impl Serialize,
    ) -> Result<(), NatsError> {
        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()),
        };
        let topic = topics::session_topic(project_id, action)?;
        client.publish_json(&topic, data).await
    }

    pub async fn publish_command(
        &self,
        command: &str,
        data: &impl Serialize,
    ) -> Result<(), NatsError> {
        let client = match &self.client {
            Some(c) => c,
            None => return Err(NatsError::NotConfigured),
        };
        let topic = topics::commands_topic(command)?;
        client.publish_json(&topic, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn without_nats_publish_event_is_noop() {
        let publisher = EventPublisher::without_nats();
        let event = ennio_core::event::OrchestratorEvent {
            id: ennio_core::id::EventId::new("test-event-1").unwrap(),
            event_type: ennio_core::event::EventType::SessionSpawned,
            priority: ennio_core::event::EventPriority::Info,
            session_id: ennio_core::id::SessionId::new("test-session-1").unwrap(),
            project_id: ennio_core::id::ProjectId::new("test-project").unwrap(),
            timestamp: chrono::Utc::now(),
            message: "test".to_owned(),
            data: serde_json::Value::Null,
        };
        let result = publisher.publish_event(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn without_nats_publish_session_event_is_noop() {
        let publisher = EventPublisher::without_nats();
        let result = publisher
            .publish_session_event("proj", "start", &serde_json::json!({}))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn without_nats_publish_command_returns_not_configured() {
        let publisher = EventPublisher::without_nats();
        let result = publisher
            .publish_command("shutdown", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, NatsError::NotConfigured),
            "expected NotConfigured, got: {err}"
        );
    }
}
