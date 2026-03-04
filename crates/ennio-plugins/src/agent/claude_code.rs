use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use ennio_core::agent::{
    Agent, AgentLaunchConfig, AgentSessionInfo, CostEstimate, PromptDelivery, WorkspaceHooksConfig,
};
use ennio_core::config::ProjectConfig;
use ennio_core::error::EnnioError;
use ennio_core::runtime::RuntimeHandle;
use ennio_core::session::{ActivityDetection, ActivityState, Session};
use tracing::debug;

pub struct ClaudeCodeAgent;

impl ClaudeCodeAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClaudeCodeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for ClaudeCodeAgent {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn process_name(&self) -> &str {
        "claude"
    }

    fn prompt_delivery(&self) -> PromptDelivery {
        PromptDelivery::Inline
    }

    fn get_launch_command(&self, config: &AgentLaunchConfig<'_>) -> String {
        let mut parts = vec!["claude".to_string()];

        if let Some(perms) = config.permissions {
            if perms == "full" {
                parts.push("--dangerously-skip-permissions".to_string());
            }
        }

        if let Some(model) = config.model {
            parts.push("--model".to_string());
            parts.push(model.to_string());
        }

        if let Some(system_prompt_file) = config.system_prompt_file {
            parts.push("--system-prompt-file".to_string());
            parts.push(system_prompt_file.to_string());
        }

        if let Some(prompt) = config.prompt {
            parts.push("-p".to_string());
            parts.push(format!("\"{}\"", prompt.replace('"', "\\\"")));
        }

        parts.join(" ")
    }

    fn get_environment(&self, _config: &AgentLaunchConfig<'_>) -> HashMap<String, String> {
        HashMap::new()
    }

    fn detect_activity(&self, terminal_output: &str) -> ActivityState {
        let trimmed = terminal_output.trim();

        if trimmed.is_empty() {
            return ActivityState::Idle;
        }

        let last_lines: Vec<&str> = trimmed.lines().rev().take(5).collect();

        for line in &last_lines {
            let lower = line.to_lowercase();
            if lower.contains("waiting for input") || lower.contains("? ") || lower.contains("y/n")
            {
                return ActivityState::WaitingInput;
            }
        }

        for line in &last_lines {
            let lower = line.to_lowercase();
            if lower.contains("$") || lower.contains("❯") || lower.contains(">>>") {
                return ActivityState::Ready;
            }
        }

        for line in &last_lines {
            let lower = line.to_lowercase();
            if lower.contains("working") || lower.contains("thinking") || lower.contains("⠋") {
                return ActivityState::Active;
            }
        }

        ActivityState::Active
    }

    async fn get_activity_state(
        &self,
        session: &Session,
        ready_threshold: Duration,
    ) -> Result<Option<ActivityDetection>, EnnioError> {
        let data_dir = match &session.workspace_path {
            Some(p) => p.join(".claude"),
            None => return Ok(None),
        };

        let jsonl_path = data_dir.join("activity.jsonl");

        if !jsonl_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&jsonl_path)
            .await
            .map_err(|e| EnnioError::Io {
                path: Some(jsonl_path),
                source: e,
            })?;

        let last_line = content.lines().rev().find(|l| !l.trim().is_empty());

        let Some(line) = last_line else {
            return Ok(None);
        };

        let entry: serde_json::Value =
            serde_json::from_str(line).map_err(|e| EnnioError::Serialization {
                message: format!("failed to parse activity entry: {e}"),
            })?;

        let state_str = entry
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("active");

        let state = match state_str {
            "idle" => ActivityState::Idle,
            "ready" => ActivityState::Ready,
            "waiting_input" => ActivityState::WaitingInput,
            "blocked" => ActivityState::Blocked,
            "exited" => ActivityState::Exited,
            _ => ActivityState::Active,
        };

        let _ready_threshold = ready_threshold;

        debug!(
            session_id = %session.id,
            state = %state,
            "detected activity state from JSONL"
        );

        Ok(Some(ActivityDetection {
            state,
            detected_at: Utc::now(),
        }))
    }

    async fn is_process_running(&self, handle: &RuntimeHandle) -> Result<bool, EnnioError> {
        let name = &handle.runtime_name;
        let output = tokio::process::Command::new("tmux")
            .args(["has-session", "-t", name])
            .output()
            .await
            .map_err(|e| EnnioError::Runtime {
                message: format!("failed to check tmux session: {e}"),
            })?;

        Ok(output.status.success())
    }

    async fn get_session_info(
        &self,
        session: &Session,
    ) -> Result<Option<AgentSessionInfo>, EnnioError> {
        let data_dir = match &session.workspace_path {
            Some(p) => p.join(".claude"),
            None => return Ok(None),
        };

        let jsonl_path = data_dir.join("session.jsonl");

        if !jsonl_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&jsonl_path)
            .await
            .map_err(|e| EnnioError::Io {
                path: Some(jsonl_path),
                source: e,
            })?;

        let mut summary = None;
        let mut cost = None;

        for line in content.lines().rev() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let entry: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if summary.is_none() {
                summary = entry
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            }

            if cost.is_none() {
                if let Some(cost_obj) = entry.get("cost") {
                    let input_tokens = cost_obj
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let output_tokens = cost_obj
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let estimated_cost_usd = cost_obj
                        .get("estimated_cost_usd")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    cost = Some(CostEstimate {
                        input_tokens,
                        output_tokens,
                        estimated_cost_usd,
                    });
                }
            }

            if summary.is_some() && cost.is_some() {
                break;
            }
        }

        Ok(Some(AgentSessionInfo { summary, cost }))
    }

    async fn get_restore_command(
        &self,
        session: &Session,
        _project: &ProjectConfig,
    ) -> Result<Option<String>, EnnioError> {
        let workspace = match &session.workspace_path {
            Some(p) => p,
            None => return Ok(None),
        };

        let cmd = format!(
            "cd {} && claude --dangerously-skip-permissions --resume",
            workspace.display()
        );
        Ok(Some(cmd))
    }

    async fn post_launch_setup(&self, _session: &Session) -> Result<(), EnnioError> {
        Ok(())
    }

    async fn setup_workspace_hooks(
        &self,
        workspace_path: &Path,
        config: &WorkspaceHooksConfig<'_>,
    ) -> Result<(), EnnioError> {
        let hooks_dir = workspace_path.join(".claude");
        tokio::fs::create_dir_all(&hooks_dir)
            .await
            .map_err(|e| EnnioError::Io {
                path: Some(hooks_dir.to_path_buf()),
                source: e,
            })?;

        let hook_script = format!(
            r#"#!/bin/sh
# Ennio metadata updater hook for session {}
# Data dir: {}
echo '{{"session_id":"{}","project":"{}","timestamp":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"}}' >> {}/activity.jsonl
"#,
            config.session_id,
            config.data_dir.display(),
            config.session_id,
            config.project_config.name,
            hooks_dir.display(),
        );

        let hook_path = hooks_dir.join("metadata-hook.sh");
        tokio::fs::write(&hook_path, &hook_script)
            .await
            .map_err(|e| EnnioError::Io {
                path: Some(hook_path.to_path_buf()),
                source: e,
            })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            tokio::fs::set_permissions(&hook_path, perms)
                .await
                .map_err(|e| EnnioError::Io {
                    path: Some(hook_path.to_path_buf()),
                    source: e,
                })?;
        }

        debug!(
            session_id = %config.session_id,
            hook = %hook_path.display(),
            "wrote metadata updater hook"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennio_core::session::ActivityState;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn detect_activity_never_panics(input in "\\PC*") {
            let agent = ClaudeCodeAgent::new();
            let _ = agent.detect_activity(&input);
        }

        #[test]
        fn detect_activity_empty_is_idle(whitespace in "[ \\t\\n]*") {
            let agent = ClaudeCodeAgent::new();
            prop_assert_eq!(agent.detect_activity(&whitespace), ActivityState::Idle);
        }

        #[test]
        fn detect_activity_waiting_input_detected(
            prefix in "\\PC{0,50}",
            marker in prop::sample::select(vec![
                "waiting for input here",
                "continue? yes",
                "proceed (y/n)",
            ]),
        ) {
            let agent = ClaudeCodeAgent::new();
            let input = format!("{prefix}\n{marker}");
            let state = agent.detect_activity(&input);
            prop_assert_eq!(state, ActivityState::WaitingInput);
        }

        #[test]
        fn detect_activity_ready_prompt_detected(
            prefix in "\\PC{0,50}",
            marker in prop::sample::select(vec!["$", "❯", ">>>"]),
        ) {
            let agent = ClaudeCodeAgent::new();
            let input = format!("{prefix}\n{marker}");
            let state = agent.detect_activity(&input);
            // Ready only if no WaitingInput markers are present
            if !input.to_lowercase().contains("waiting for input")
                && !input.contains("? ")
                && !input.to_lowercase().contains("y/n")
            {
                prop_assert_eq!(state, ActivityState::Ready);
            }
        }

        #[test]
        fn detect_activity_working_detected(
            prefix in "\\PC{0,30}",
            marker in prop::sample::select(vec!["working", "thinking", "⠋"]),
        ) {
            let agent = ClaudeCodeAgent::new();
            let input = format!("{prefix}\n{marker}");
            let state = agent.detect_activity(&input);
            // Active only if no higher-priority markers are present
            let lower = input.to_lowercase();
            if !lower.contains("waiting for input")
                && !input.contains("? ")
                && !lower.contains("y/n")
                && !input.contains('$')
                && !input.contains('❯')
                && !input.contains(">>>")
            {
                prop_assert_eq!(state, ActivityState::Active);
            }
        }
    }
}
