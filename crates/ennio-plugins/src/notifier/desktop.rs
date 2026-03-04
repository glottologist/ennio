use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::event::OrchestratorEvent;
use ennio_core::notifier::{Notifier, NotifyAction, NotifyContext};
use tokio::process::Command;
use tracing::debug;

pub struct DesktopNotifier;

impl DesktopNotifier {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DesktopNotifier {
    fn default() -> Self {
        Self::new()
    }
}

async fn send_desktop_notification(title: &str, body: &str) -> Result<(), EnnioError> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("notify-send")
            .args([title, body])
            .output()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("notify-send failed: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnnioError::Notifier {
                message: format!("notify-send exited with error: {stderr}"),
            });
        }
    }

    #[cfg(target_os = "macos")]
    {
        let safe = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            r#"display notification "{}" with title "{}""#,
            safe(body),
            safe(title)
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .await
            .map_err(|e| EnnioError::Notifier {
                message: format!("osascript failed: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnnioError::Notifier {
                message: format!("osascript exited with error: {stderr}"),
            });
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _title = title;
        let _body = body;
        return Err(EnnioError::Notifier {
            message: "desktop notifications not supported on this platform".to_owned(),
        });
    }

    Ok(())
}

#[async_trait]
impl Notifier for DesktopNotifier {
    fn name(&self) -> &str {
        "desktop"
    }

    async fn notify(&self, event: &OrchestratorEvent) -> Result<(), EnnioError> {
        debug!(
            event_type = %event.event_type,
            session_id = %event.session_id,
            "sending desktop notification"
        );

        let title = format!("Ennio: {}", event.event_type);
        let body = format!("[{}] {}", event.session_id, event.message);
        send_desktop_notification(&title, &body).await
    }

    async fn notify_with_actions(
        &self,
        event: &OrchestratorEvent,
        _actions: &[NotifyAction],
    ) -> Result<(), EnnioError> {
        self.notify(event).await
    }

    async fn post(&self, context: &NotifyContext, message: &str) -> Result<(), EnnioError> {
        let title = format!("Ennio: {}", context.project_name);
        let body = format!("[{}] {message}", context.session_id);
        send_desktop_notification(&title, &body).await
    }
}
