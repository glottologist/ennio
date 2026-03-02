use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::runtime::{
    AttachInfo, Runtime, RuntimeCreateConfig, RuntimeHandle, RuntimeMetrics,
};
use tokio::process::Command;
use tracing::debug;

pub struct ProcessRuntime;

impl ProcessRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProcessRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Runtime for ProcessRuntime {
    fn name(&self) -> &str {
        "process"
    }

    async fn create(&self, config: &RuntimeCreateConfig) -> Result<RuntimeHandle, EnnioError> {
        debug!(
            session_id = %config.session_id,
            command = %config.launch_command,
            "spawning local process"
        );

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(&config.launch_command)
            .current_dir(&config.cwd);

        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let child = cmd.spawn().map_err(|e| EnnioError::Runtime {
            message: format!("failed to spawn process: {e}"),
        })?;

        let pid = child.id().ok_or_else(|| EnnioError::Runtime {
            message: "spawned process has no PID".to_string(),
        })?;

        let mut data = HashMap::new();
        data.insert(
            "runtime_type".to_string(),
            serde_json::Value::String("process".to_string()),
        );
        data.insert(
            "pid".to_string(),
            serde_json::Value::Number(serde_json::Number::from(pid)),
        );

        Ok(RuntimeHandle {
            id: config.session_id.to_string(),
            runtime_name: config.session_name.to_string(),
            data,
        })
    }

    async fn destroy(&self, handle: &RuntimeHandle) -> Result<(), EnnioError> {
        let pid = extract_pid(handle)?;

        debug!(pid = pid, "killing process");

        let output = Command::new("kill")
            .arg(pid.to_string())
            .output()
            .await
            .map_err(|e| EnnioError::Runtime {
                message: format!("failed to kill process {pid}: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnnioError::Runtime {
                message: format!("kill failed for pid {pid}: {stderr}"),
            });
        }

        Ok(())
    }

    async fn send_message(
        &self,
        _handle: &RuntimeHandle,
        _message: &str,
    ) -> Result<(), EnnioError> {
        Err(EnnioError::Plugin {
            plugin: "process".to_string(),
            message: "process runtime does not support send_message".to_string(),
        })
    }

    async fn get_output(&self, _handle: &RuntimeHandle, _lines: u32) -> Result<String, EnnioError> {
        Err(EnnioError::Plugin {
            plugin: "process".to_string(),
            message: "process runtime does not support get_output".to_string(),
        })
    }

    async fn is_alive(&self, handle: &RuntimeHandle) -> Result<bool, EnnioError> {
        let pid = extract_pid(handle)?;

        let output = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .await
            .map_err(|e| EnnioError::Runtime {
                message: format!("failed to check process {pid}: {e}"),
            })?;

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
        let pid = extract_pid(handle)?;
        Ok(AttachInfo {
            command: format!("# Process PID: {pid}"),
            url: None,
            instructions: Some("Process runtime does not support direct attachment".to_string()),
        })
    }
}

fn extract_pid(handle: &RuntimeHandle) -> Result<u32, EnnioError> {
    handle
        .data
        .get("pid")
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| EnnioError::Runtime {
            message: "runtime handle missing pid".to_string(),
        })
}
