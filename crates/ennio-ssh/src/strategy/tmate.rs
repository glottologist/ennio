use std::collections::HashMap;

use async_trait::async_trait;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};
use tracing::{debug, warn};

use super::SshSessionStrategy;
use crate::client::SshClient;
use crate::error::SshError;
use crate::shell;

const TMATE_READY_POLL_INTERVAL_MS: u64 = 500;
const TMATE_READY_MAX_ATTEMPTS: u32 = 20;

pub struct TmateStrategy;

impl TmateStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TmateStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SshSessionStrategy for TmateStrategy {
    async fn create_session(
        &self,
        client: &SshClient,
        config: &RuntimeCreateConfig,
    ) -> Result<RuntimeHandle, SshError> {
        let name = shell::escape(&config.session_name);
        let command = shell::escape(&config.launch_command);
        let cwd = shell::escape(&config.cwd);
        let socket = format!("/tmp/tmate-{}.sock", &config.session_name);
        let escaped_socket = shell::escape(&socket);

        let mut env_exports = String::new();
        for (key, value) in &config.env {
            env_exports.push_str(&format!(
                "export {}={}; ",
                shell::escape(key),
                shell::escape(value)
            ));
        }

        let launch_cmd = format!(
            "cd {cwd} && {env_exports}tmate -S {escaped_socket} new-session -d -s {name} {command}"
        );

        debug!(session_name = %config.session_name, "creating tmate session");

        let output = client.exec(&launch_cmd).await?;
        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: launch_cmd,
                    message: format!(
                        "tmate new-session failed (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
        }

        let (web_url, ssh_url) = poll_tmate_urls(client, &socket, &config.session_name).await?;

        let mut data = HashMap::new();
        data.insert(
            "runtime_type".to_string(),
            serde_json::Value::String("tmate".to_string()),
        );
        data.insert("socket".to_string(), serde_json::Value::String(socket));
        data.insert("web_url".to_string(), serde_json::Value::String(web_url));
        data.insert("ssh_url".to_string(), serde_json::Value::String(ssh_url));

        Ok(RuntimeHandle {
            id: config.session_id.to_string(),
            runtime_name: config.session_name.to_string(),
            data,
        })
    }

    async fn destroy_session(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
    ) -> Result<(), SshError> {
        let name = shell::escape(&handle.runtime_name);
        let socket = extract_data_str(&handle.data, "socket")?;
        let escaped_socket = shell::escape(socket);

        let cmd = format!("tmate -S {escaped_socket} kill-session -t {name}");

        debug!(session_name = %handle.runtime_name, "destroying tmate session");

        let output = client.exec(&cmd).await?;
        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: cmd,
                    message: format!(
                        "tmate kill-session failed (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
        }

        let cleanup_cmd = format!("rm -f {escaped_socket}");
        if let Err(e) = client.exec(&cleanup_cmd).await {
            warn!("cleanup failed for tmate socket: {e}");
        }

        Ok(())
    }

    async fn send_message(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
        message: &str,
    ) -> Result<(), SshError> {
        let name = shell::escape(&handle.runtime_name);
        let socket = extract_data_str(&handle.data, "socket")?;
        let escaped_socket = shell::escape(socket);
        let escaped_msg = shell::escape(message);

        let cmd = format!("tmate -S {escaped_socket} send-keys -t {name} {escaped_msg} Enter");

        debug!(session_name = %handle.runtime_name, "sending message via tmate");

        let output = client.exec(&cmd).await?;
        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: cmd,
                    message: format!("tmate send-keys failed (exit {}): {}", code, output.stderr),
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
        let name = shell::escape(&handle.runtime_name);
        let socket = extract_data_str(&handle.data, "socket")?;
        let escaped_socket = shell::escape(socket);

        let cmd = format!("tmate -S {escaped_socket} capture-pane -t {name} -p -S -{lines}");

        let output = client.exec(&cmd).await?;
        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: cmd,
                    message: format!(
                        "tmate capture-pane failed (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
        }

        Ok(output.stdout)
    }

    async fn is_alive(&self, client: &SshClient, handle: &RuntimeHandle) -> Result<bool, SshError> {
        let name = shell::escape(&handle.runtime_name);
        let socket = extract_data_str(&handle.data, "socket")?;
        let escaped_socket = shell::escape(socket);

        let cmd = format!("tmate -S {escaped_socket} has-session -t {name}");

        let output = client.exec(&cmd).await?;
        Ok(output.exit_code == Some(0))
    }
}

async fn poll_tmate_urls(
    client: &SshClient,
    socket: &str,
    name: &str,
) -> Result<(String, String), SshError> {
    let escaped_socket = shell::escape(socket);
    let web_cmd = format!("tmate -S {escaped_socket} display -p '#{{tmate_web}}'");
    let ssh_cmd = format!("tmate -S {escaped_socket} display -p '#{{tmate_ssh}}'");

    for attempt in 0..TMATE_READY_MAX_ATTEMPTS {
        debug!(session_name = %name, attempt, "polling tmate URLs");

        let web_output = client.exec(&web_cmd).await?;
        let ssh_output = client.exec(&ssh_cmd).await?;

        let web_url = web_output.stdout.trim().to_string();
        let ssh_url = ssh_output.stdout.trim().to_string();

        if !web_url.is_empty() && !ssh_url.is_empty() {
            return Ok((web_url, ssh_url));
        }

        tokio::time::sleep(std::time::Duration::from_millis(
            TMATE_READY_POLL_INTERVAL_MS,
        ))
        .await;
    }

    Err(SshError::Timeout {
        duration: std::time::Duration::from_millis(
            u64::from(TMATE_READY_MAX_ATTEMPTS) * TMATE_READY_POLL_INTERVAL_MS,
        ),
    })
}

fn extract_data_str<'a>(
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
