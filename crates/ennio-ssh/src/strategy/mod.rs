mod remote_control;
mod tmate;
mod tmux;

pub use remote_control::RemoteControlStrategy;
pub use tmate::TmateStrategy;
pub use tmux::TmuxStrategy;

use async_trait::async_trait;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};

use crate::client::SshClient;
use crate::config::SshStrategy;
use crate::error::SshError;

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

pub fn create_strategy(strategy: SshStrategy) -> Box<dyn SshSessionStrategy> {
    match strategy {
        SshStrategy::Tmux => Box::new(TmuxStrategy::new()),
        SshStrategy::Tmate => Box::new(TmateStrategy::new()),
        SshStrategy::RemoteControl => Box::new(RemoteControlStrategy::new()),
        SshStrategy::Node => Box::new(TmuxStrategy::new()),
    }
}
