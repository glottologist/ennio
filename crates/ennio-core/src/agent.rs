use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::config::ProjectConfig;
use crate::error::EnnioError;
use crate::id::SessionId;
use crate::runtime::RuntimeHandle;
use crate::session::{ActivityDetection, ActivityState, Session};
use crate::tracker::Issue;

#[derive(Debug, Clone)]
pub struct AgentLaunchConfig<'a> {
    pub session_id: &'a SessionId,
    pub project_config: &'a ProjectConfig,
    pub issue: Option<&'a Issue>,
    pub prompt: Option<&'a str>,
    pub permissions: Option<&'a str>,
    pub model: Option<&'a str>,
    pub system_prompt: Option<&'a str>,
    pub system_prompt_file: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionInfo {
    pub summary: Option<String>,
    pub cost: Option<CostEstimate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PromptDelivery {
    Inline,
    PostLaunch,
}

#[derive(Debug, Clone)]
pub struct WorkspaceHooksConfig<'a> {
    pub session_id: &'a SessionId,
    pub data_dir: &'a Path,
    pub project_config: &'a ProjectConfig,
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;

    fn process_name(&self) -> &str;

    fn prompt_delivery(&self) -> PromptDelivery;

    fn get_launch_command(&self, config: &AgentLaunchConfig<'_>) -> String;

    fn get_environment(&self, config: &AgentLaunchConfig<'_>) -> HashMap<String, String>;

    fn detect_activity(&self, terminal_output: &str) -> ActivityState;

    async fn get_activity_state(
        &self,
        session: &Session,
        ready_threshold: Duration,
    ) -> Result<Option<ActivityDetection>, EnnioError>;

    async fn is_process_running(&self, handle: &RuntimeHandle) -> Result<bool, EnnioError>;

    async fn get_session_info(
        &self,
        session: &Session,
    ) -> Result<Option<AgentSessionInfo>, EnnioError>;

    async fn get_restore_command(
        &self,
        session: &Session,
        project: &ProjectConfig,
    ) -> Result<Option<String>, EnnioError>;

    async fn post_launch_setup(&self, session: &Session) -> Result<(), EnnioError>;

    async fn setup_workspace_hooks(
        &self,
        workspace_path: &Path,
        config: &WorkspaceHooksConfig<'_>,
    ) -> Result<(), EnnioError>;
}
