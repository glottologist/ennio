use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::runtime::{
    AttachInfo, Runtime, RuntimeCreateConfig, RuntimeHandle, RuntimeMetrics,
};
use tokio::process::Command;
use tracing::debug;

const SEND_KEYS_MAX_LEN: usize = 200;

pub struct TmuxRuntime;

impl TmuxRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TmuxRuntime {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_tmux(args: &[&str]) -> Result<std::process::Output, EnnioError> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .await
        .map_err(|e| EnnioError::Runtime {
            message: format!("failed to execute tmux: {e}"),
        })?;
    Ok(output)
}

fn check_exit(output: &std::process::Output, context: &str) -> Result<(), EnnioError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(EnnioError::Runtime {
            message: format!("{context}: {stderr}"),
        });
    }
    Ok(())
}

#[async_trait]
impl Runtime for TmuxRuntime {
    fn name(&self) -> &str {
        "tmux"
    }

    async fn create(&self, config: &RuntimeCreateConfig) -> Result<RuntimeHandle, EnnioError> {
        let name = &config.session_name;
        let command = &config.launch_command;

        debug!(session_name = %name, "creating local tmux session");

        let output =
            run_tmux(&["new-session", "-d", "-s", name, "-c", &config.cwd, command]).await?;

        check_exit(&output, "tmux new-session failed")?;

        let mut data = HashMap::new();
        data.insert(
            "runtime_type".to_string(),
            serde_json::Value::String("tmux".to_string()),
        );

        Ok(RuntimeHandle {
            id: config.session_id.to_string(),
            runtime_name: name.to_string(),
            data,
        })
    }

    async fn destroy(&self, handle: &RuntimeHandle) -> Result<(), EnnioError> {
        let name = &handle.runtime_name;

        debug!(session_name = %name, "destroying local tmux session");

        let output = run_tmux(&["kill-session", "-t", name]).await?;
        check_exit(&output, "tmux kill-session failed")?;

        Ok(())
    }

    async fn send_message(&self, handle: &RuntimeHandle, message: &str) -> Result<(), EnnioError> {
        let name = &handle.runtime_name;

        if message.len() > SEND_KEYS_MAX_LEN {
            send_via_buffer(name, message).await
        } else {
            send_via_keys(name, message).await
        }
    }

    async fn get_output(&self, handle: &RuntimeHandle, lines: u32) -> Result<String, EnnioError> {
        let name = &handle.runtime_name;
        let lines_arg = format!("-{lines}");

        let output = run_tmux(&["capture-pane", "-t", name, "-p", "-S", &lines_arg]).await?;
        check_exit(&output, "tmux capture-pane failed")?;

        String::from_utf8(output.stdout).map_err(|e| EnnioError::Runtime {
            message: format!("invalid UTF-8 in tmux output: {e}"),
        })
    }

    async fn is_alive(&self, handle: &RuntimeHandle) -> Result<bool, EnnioError> {
        let name = &handle.runtime_name;
        let output = run_tmux(&["has-session", "-t", name]).await?;
        Ok(output.status.success())
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
            command: format!("tmux attach-session -t {name}"),
            url: None,
            instructions: None,
        })
    }
}

async fn send_via_keys(name: &str, message: &str) -> Result<(), EnnioError> {
    let escaped = message.replace('\'', "'\\''");

    debug!(session_name = %name, len = message.len(), "sending via send-keys");

    let output = run_tmux(&["send-keys", "-t", name, &escaped, "Enter"]).await?;
    check_exit(&output, "tmux send-keys failed")?;

    Ok(())
}

async fn send_via_buffer(name: &str, message: &str) -> Result<(), EnnioError> {
    let tmp = tempfile::NamedTempFile::new().map_err(|e| EnnioError::Io {
        path: None,
        source: e,
    })?;
    let tmp_path = tmp.path().to_string_lossy().into_owned();

    debug!(session_name = %name, len = message.len(), "sending via load-buffer");

    tokio::fs::write(&tmp_path, message.as_bytes())
        .await
        .map_err(|e| EnnioError::Io {
            path: Some(tmp_path.clone().into()), // clone: tmp_path used after this for tmux commands
            source: e,
        })?;

    let output = run_tmux(&["load-buffer", &tmp_path]).await?;
    check_exit(&output, "tmux load-buffer failed")?;

    let output = run_tmux(&["paste-buffer", "-t", name]).await?;
    check_exit(&output, "tmux paste-buffer failed")?;

    let output = run_tmux(&["send-keys", "-t", name, "Enter"]).await?;
    check_exit(&output, "tmux send-keys failed")?;

    Ok(())
}
