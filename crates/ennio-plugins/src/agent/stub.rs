use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use ennio_core::agent::{
    Agent, AgentLaunchConfig, AgentSessionInfo, PromptDelivery, WorkspaceHooksConfig,
};
use ennio_core::config::ProjectConfig;
use ennio_core::error::EnnioError;
use ennio_core::runtime::RuntimeHandle;
use ennio_core::session::{ActivityDetection, ActivityState, Session};

pub struct StubAgent {
    agent_name: &'static str,
    process: &'static str,
}

impl StubAgent {
    pub const fn new(agent_name: &'static str, process: &'static str) -> Self {
        Self {
            agent_name,
            process,
        }
    }

    fn not_implemented(&self, operation: &str) -> EnnioError {
        EnnioError::Plugin {
            plugin: self.agent_name.to_string(),
            message: format!("{} agent {operation} not yet implemented", self.agent_name),
        }
    }
}

#[async_trait]
impl Agent for StubAgent {
    fn name(&self) -> &str {
        self.agent_name
    }

    fn process_name(&self) -> &str {
        self.process
    }

    fn prompt_delivery(&self) -> PromptDelivery {
        PromptDelivery::Inline
    }

    fn get_launch_command(&self, config: &AgentLaunchConfig<'_>) -> String {
        let mut parts = vec![self.agent_name.to_string()];

        if let Some(prompt) = config.prompt {
            if self.agent_name == "aider" {
                parts.push("--message".to_string());
            }
            parts.push(format!("\"{}\"", prompt.replace('"', "\\\"")));
        }

        parts.join(" ")
    }

    fn get_environment(&self, _config: &AgentLaunchConfig<'_>) -> HashMap<String, String> {
        HashMap::new()
    }

    fn detect_activity(&self, terminal_output: &str) -> ActivityState {
        if terminal_output.trim().is_empty() {
            ActivityState::Idle
        } else {
            ActivityState::Active
        }
    }

    async fn get_activity_state(
        &self,
        _session: &Session,
        _ready_threshold: Duration,
    ) -> Result<Option<ActivityDetection>, EnnioError> {
        Err(self.not_implemented("activity detection"))
    }

    async fn is_process_running(&self, _handle: &RuntimeHandle) -> Result<bool, EnnioError> {
        Err(self.not_implemented("process check"))
    }

    async fn get_session_info(
        &self,
        _session: &Session,
    ) -> Result<Option<AgentSessionInfo>, EnnioError> {
        Err(self.not_implemented("session info"))
    }

    async fn get_restore_command(
        &self,
        _session: &Session,
        _project: &ProjectConfig,
    ) -> Result<Option<String>, EnnioError> {
        Err(self.not_implemented("restore"))
    }

    async fn post_launch_setup(&self, _session: &Session) -> Result<(), EnnioError> {
        Ok(())
    }

    async fn setup_workspace_hooks(
        &self,
        _workspace_path: &Path,
        _config: &WorkspaceHooksConfig<'_>,
    ) -> Result<(), EnnioError> {
        Ok(())
    }
}
