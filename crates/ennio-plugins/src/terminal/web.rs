use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::session::Session;
use ennio_core::terminal::Terminal;
use tracing::debug;
use url::Url;

pub struct WebTerminal {
    base_url: String,
}

impl WebTerminal {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl Terminal for WebTerminal {
    fn name(&self) -> &str {
        "web"
    }

    async fn open_session(&self, session: &Session) -> Result<(), EnnioError> {
        let mut base = Url::parse(&self.base_url).map_err(|e| EnnioError::Plugin {
            plugin: "web-terminal".to_owned(),
            message: format!("invalid base URL: {e}"),
        })?;
        base.path_segments_mut()
            .map_err(|()| EnnioError::Plugin {
                plugin: "web-terminal".to_owned(),
                message: "base URL cannot-be-a-base".to_owned(),
            })?
            .push("sessions")
            .push(&session.id.to_string());
        let url = base;
        debug!(url = %url, "opening web terminal session");

        #[cfg(target_os = "linux")]
        {
            tokio::process::Command::new("xdg-open")
                .arg(url.as_str())
                .spawn()
                .map_err(|e| EnnioError::Plugin {
                    plugin: "web-terminal".to_owned(),
                    message: format!("failed to open browser: {e}"),
                })?;
        }

        #[cfg(target_os = "macos")]
        {
            tokio::process::Command::new("open")
                .arg(url.as_str())
                .spawn()
                .map_err(|e| EnnioError::Plugin {
                    plugin: "web-terminal".to_owned(),
                    message: format!("failed to open browser: {e}"),
                })?;
        }

        Ok(())
    }

    async fn open_all(&self, sessions: &[Session]) -> Result<(), EnnioError> {
        for session in sessions {
            self.open_session(session).await?;
        }
        Ok(())
    }

    async fn is_session_open(&self, _session: &Session) -> Result<bool, EnnioError> {
        Ok(false)
    }
}
