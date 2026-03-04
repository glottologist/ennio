use std::collections::HashMap;

use async_trait::async_trait;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};
use tracing::{debug, warn};

use super::{SshSessionStrategy, build_env_exports, extract_data_str};
use crate::client::SshClient;
use crate::error::SshError;
use crate::shell;

const SESSION_URL_POLL_INTERVAL_MS: u64 = 500;
const SESSION_URL_MAX_ATTEMPTS: u32 = 40;

pub struct RemoteControlStrategy;

impl RemoteControlStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RemoteControlStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SshSessionStrategy for RemoteControlStrategy {
    async fn create_session(
        &self,
        client: &SshClient,
        config: &RuntimeCreateConfig,
    ) -> Result<RuntimeHandle, SshError> {
        let name = &config.session_name;
        let cwd = shell::escape(&config.cwd);

        let env_exports = build_env_exports(&config.env);

        let log_file = format!("/tmp/ennio-rc-{name}.log");
        let pid_file = format!("/tmp/ennio-rc-{name}.pid");
        let escaped_log = shell::escape(&log_file);
        let escaped_pid = shell::escape(&pid_file);

        let launch_cmd = format!(
            "cd {cwd} && {env_exports}nohup claude remote-control > {escaped_log} 2>&1 & echo $! > {escaped_pid}"
        );

        debug!(session_name = %name, "creating remote-control session");

        let output = client.exec(&launch_cmd).await?;
        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: launch_cmd,
                    message: format!(
                        "failed to start claude remote-control (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
        }

        let session_url = poll_session_url(client, &log_file, name).await?;

        let mut data = HashMap::new();
        data.insert(
            "runtime_type".to_string(),
            serde_json::Value::String("remote_control".to_string()),
        );
        data.insert(
            "session_url".to_string(),
            serde_json::Value::String(session_url),
        );
        data.insert("log_file".to_string(), serde_json::Value::String(log_file));
        data.insert("pid_file".to_string(), serde_json::Value::String(pid_file));

        Ok(RuntimeHandle {
            id: config.session_id.to_string(),
            runtime_name: name.to_string(),
            data,
        })
    }

    async fn destroy_session(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
    ) -> Result<(), SshError> {
        let name = &handle.runtime_name;
        let pid_file = extract_data_str(&handle.data, "pid_file")?;
        let log_file = extract_data_str(&handle.data, "log_file")?;
        let escaped_pid = shell::escape(pid_file);
        let escaped_log = shell::escape(log_file);

        let kill_cmd =
            format!("if [ -f {escaped_pid} ]; then kill $(cat {escaped_pid}) 2>/dev/null; fi");

        debug!(session_name = %name, "destroying remote-control session");

        if let Err(e) = client.exec(&kill_cmd).await {
            warn!("kill command failed for remote-control session: {e}");
        }

        let cleanup_cmd = format!("rm -f {escaped_pid} {escaped_log}");
        if let Err(e) = client.exec(&cleanup_cmd).await {
            warn!("cleanup failed for remote-control session: {e}");
        }

        Ok(())
    }

    async fn send_message(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
        message: &str,
    ) -> Result<(), SshError> {
        let name = &handle.runtime_name;
        let session_url = extract_data_str(&handle.data, "session_url")?;

        let payload = serde_json::json!({
            "type": "message",
            "content": message
        })
        .to_string();
        let escaped_payload = shell::escape(&payload);
        let escaped_url = shell::escape(session_url);

        let cmd = format!(
            "curl -s -X POST -H 'Content-Type: application/json' -d {escaped_payload} {escaped_url}/message"
        );

        debug!(session_name = %name, "sending message via remote-control HTTP API");

        let output = client.exec(&cmd).await?;
        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: "curl POST /message".to_string(),
                    message: format!(
                        "remote-control send failed (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
        }

        Ok(())
    }

    async fn get_output(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
        lines: u32,
    ) -> Result<String, SshError> {
        let name = &handle.runtime_name;
        let session_url = extract_data_str(&handle.data, "session_url")?;
        let escaped_url = shell::escape(session_url);

        let cmd = format!("curl -s {escaped_url}/output?lines={lines}");

        debug!(session_name = %name, lines, "getting output via remote-control HTTP API");

        let output = client.exec(&cmd).await?;
        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: "curl GET /output".to_string(),
                    message: format!(
                        "remote-control get_output failed (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
        }

        Ok(output.stdout)
    }

    async fn is_alive(&self, client: &SshClient, handle: &RuntimeHandle) -> Result<bool, SshError> {
        let pid_file = extract_data_str(&handle.data, "pid_file")?;
        let escaped_pid = shell::escape(pid_file);

        let cmd = format!(
            "if [ -f {escaped_pid} ]; then kill -0 $(cat {escaped_pid}) 2>/dev/null && echo alive; fi"
        );

        let output = client.exec(&cmd).await?;
        Ok(output.stdout.trim() == "alive")
    }
}

async fn poll_session_url(
    client: &SshClient,
    log_file: &str,
    name: &str,
) -> Result<String, SshError> {
    let escaped_log = shell::escape(log_file);
    let grep_cmd = format!("grep -oP 'http://[^ ]+' {escaped_log} | head -1");

    for attempt in 0..SESSION_URL_MAX_ATTEMPTS {
        debug!(session_name = %name, attempt, "polling for remote-control session URL");

        let output = client.exec(&grep_cmd).await?;
        let url = output.stdout.trim().to_string();

        if !url.is_empty() {
            return Ok(url);
        }

        tokio::time::sleep(std::time::Duration::from_millis(
            SESSION_URL_POLL_INTERVAL_MS,
        ))
        .await;
    }

    Err(SshError::Timeout {
        duration: std::time::Duration::from_millis(
            u64::from(SESSION_URL_MAX_ATTEMPTS) * SESSION_URL_POLL_INTERVAL_MS,
        ),
    })
}
