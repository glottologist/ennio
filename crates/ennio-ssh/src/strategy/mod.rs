mod remote_control;
mod tmate;
mod tmux;

pub use remote_control::RemoteControlStrategy;
pub use tmate::TmateStrategy;
pub use tmux::TmuxStrategy;

use std::collections::HashMap;

use async_trait::async_trait;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};

use crate::client::SshClient;
use crate::error::SshError;
use crate::shell;
use ennio_core::config::SshStrategyConfig;

#[async_trait]
pub trait SshSessionStrategy: Send + Sync {
    async fn create_session(
        &self,
        client: &SshClient,
        config: &RuntimeCreateConfig,
    ) -> Result<RuntimeHandle, SshError>;

    async fn destroy_session(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
    ) -> Result<(), SshError>;

    async fn send_message(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
        message: &str,
    ) -> Result<(), SshError>;

    async fn get_output(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
        lines: u32,
    ) -> Result<String, SshError>;

    async fn is_alive(&self, client: &SshClient, handle: &RuntimeHandle) -> Result<bool, SshError>;
}

pub fn create_strategy(strategy: SshStrategyConfig) -> Box<dyn SshSessionStrategy> {
    match strategy {
        SshStrategyConfig::Tmux => Box::new(TmuxStrategy::new()),
        SshStrategyConfig::Tmate => Box::new(TmateStrategy::new()),
        SshStrategyConfig::RemoteControl => Box::new(RemoteControlStrategy::new()),
        SshStrategyConfig::Node => Box::new(TmuxStrategy::new()),
    }
}

pub(super) fn extract_data_str<'a>(
    data: &'a HashMap<String, serde_json::Value>,
    key: &str,
) -> Result<&'a str, SshError> {
    data.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| SshError::Execution {
            command: String::new(),
            message: format!("missing '{key}' in runtime handle data"),
        })
}

pub(super) fn build_env_exports(env: &HashMap<String, String>) -> String {
    let mut exports = String::new();
    for (key, value) in env {
        exports.push_str(&format!(
            "export {}={}; ",
            shell::escape(key),
            shell::escape(value)
        ));
    }
    exports
}
