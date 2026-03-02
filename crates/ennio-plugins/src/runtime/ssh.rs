use std::time::Duration;

use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::runtime::{
    AttachInfo, Runtime, RuntimeCreateConfig, RuntimeHandle, RuntimeMetrics,
};
use ennio_ssh::SshClient;
use ennio_ssh::strategy::SshSessionStrategy;
use tracing::debug;

pub struct SshRuntime {
    client: SshClient,
    strategy: Box<dyn SshSessionStrategy>,
}

impl SshRuntime {
    pub fn new(client: SshClient, strategy: Box<dyn SshSessionStrategy>) -> Self {
        Self { client, strategy }
    }
}

#[async_trait]
impl Runtime for SshRuntime {
    fn name(&self) -> &str {
        "ssh"
    }

    async fn create(&self, config: &RuntimeCreateConfig) -> Result<RuntimeHandle, EnnioError> {
        debug!(
            session_name = %config.session_name,
            "creating SSH session via strategy"
        );

        self.strategy
            .create_session(&self.client, config)
            .await
            .map_err(EnnioError::from)
    }

    async fn destroy(&self, handle: &RuntimeHandle) -> Result<(), EnnioError> {
        debug!(
            session_name = %handle.runtime_name,
            "destroying SSH session via strategy"
        );

        self.strategy
            .destroy_session(&self.client, handle)
            .await
            .map_err(EnnioError::from)
    }

    async fn send_message(&self, handle: &RuntimeHandle, message: &str) -> Result<(), EnnioError> {
        self.strategy
            .send_message(&self.client, handle, message)
            .await
            .map_err(EnnioError::from)
    }

    async fn get_output(&self, handle: &RuntimeHandle, lines: u32) -> Result<String, EnnioError> {
        self.strategy
            .get_output(&self.client, handle, lines)
            .await
            .map_err(EnnioError::from)
    }

    async fn is_alive(&self, handle: &RuntimeHandle) -> Result<bool, EnnioError> {
        self.strategy
            .is_alive(&self.client, handle)
            .await
            .map_err(EnnioError::from)
    }

    async fn get_metrics(&self, _handle: &RuntimeHandle) -> Result<RuntimeMetrics, EnnioError> {
        Ok(RuntimeMetrics {
            uptime: Duration::ZERO,
            cpu_percent: None,
            memory_bytes: None,
        })
    }

    async fn get_attach_info(&self, handle: &RuntimeHandle) -> Result<AttachInfo, EnnioError> {
        let name = &handle.runtime_name;
        Ok(AttachInfo {
            command: format!("ssh <host> -t tmux attach-session -t {name}"),
            url: None,
            instructions: Some("Connect via SSH and attach to the session".to_string()),
        })
    }
}
