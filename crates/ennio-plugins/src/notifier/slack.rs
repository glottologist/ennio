use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::event::OrchestratorEvent;
use ennio_core::notifier::{Notifier, NotifyAction, NotifyContext};
use reqwest::Client;
use tracing::debug;

pub struct SlackNotifier {
    client: Client,
    webhook_url: String,
    channel: Option<String>,
}

impl SlackNotifier {
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            webhook_url: webhook_url.into(),
            channel: None,
        }
    }

    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    fn format_event_text(&self, event: &OrchestratorEvent) -> String {
        format!(
            "*[{}]* {} | `{}` — {}",
            event.priority, event.event_type, event.session_id, event.message
        )
    }
}

#[async_trait]
impl Notifier for SlackNotifier {
    fn name(&self) -> &str {
        "slack"
    }

    async fn notify(&self, event: &OrchestratorEvent) -> Result<(), EnnioError> {
        debug!(
            event_type = %event.event_type,
            session_id = %event.session_id,
            "sending Slack notification"
        );

        let text = self.format_event_text(event);

        let mut payload = serde_json::json!({ "text": text });
        if let Some(ref channel) = self.channel {
            payload["channel"] = serde_json::Value::String(channel.to_owned());
        }

        self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("Slack webhook failed: {e}"),
            })?;

        Ok(())
    }

    async fn notify_with_actions(
        &self,
        event: &OrchestratorEvent,
        actions: &[NotifyAction],
    ) -> Result<(), EnnioError> {
        let text = self.format_event_text(event);

        let action_blocks: Vec<serde_json::Value> = actions
            .iter()
            .filter_map(|a| {
                a.url.as_ref().map(|url| {
                    serde_json::json!({
                        "type": "button",
                        "text": { "type": "plain_text", "text": a.label },
                        "url": url,
                    })
                })
            })
            .collect();

        let mut blocks = vec![serde_json::json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": text },
        })];

        if !action_blocks.is_empty() {
            blocks.push(serde_json::json!({
                "type": "actions",
                "elements": action_blocks,
            }));
        }

        let mut payload = serde_json::json!({
            "text": text,
            "blocks": blocks,
        });
        if let Some(ref channel) = self.channel {
            payload["channel"] = serde_json::Value::String(channel.to_owned());
        }

        self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("Slack webhook failed: {e}"),
            })?;

        Ok(())
    }

    async fn post(&self, context: &NotifyContext, message: &str) -> Result<(), EnnioError> {
        let text = format!(
            "*{}* | `{}` — {}",
            context.project_name, context.session_id, message
        );

        let mut payload = serde_json::json!({ "text": text });
        if let Some(ref channel) = self.channel {
            payload["channel"] = serde_json::Value::String(channel.to_owned());
        }

        self.client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("Slack webhook failed: {e}"),
            })?;

        Ok(())
    }
}
