use ennio_core::event::OrchestratorEvent;
use serde::Serialize;

use crate::client::NatsClient;
use crate::error::NatsError;
use crate::topics;

pub struct EventPublisher {
    client: NatsClient,
}

impl EventPublisher {
    pub fn new(client: NatsClient) -> Self {
        Self { client }
    }

    pub async fn publish_event(&self, event: &OrchestratorEvent) -> Result<(), NatsError> {
        let topic = topics::topic_for_event_type(event.event_type, event.project_id.as_str())?;
        self.client.publish_json(&topic, event).await
    }

    pub async fn publish_session_event(
        &self,
        project_id: &str,
        action: &str,
        data: &impl Serialize,
    ) -> Result<(), NatsError> {
        let topic = topics::session_topic(project_id, action)?;
        self.client.publish_json(&topic, data).await
    }

    pub async fn publish_command(
        &self,
        command: &str,
        data: &impl Serialize,
    ) -> Result<(), NatsError> {
        let topic = topics::commands_topic(command)?;
        self.client.publish_json(&topic, data).await
    }
}
