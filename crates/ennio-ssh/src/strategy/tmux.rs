use std::collections::HashMap;

use async_trait::async_trait;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};
use tracing::debug;

use super::{SshSessionStrategy, build_env_exports};
use crate::client::SshClient;
use crate::error::SshError;
use crate::shell;

const SEND_KEYS_MAX_LEN: usize = 200;

pub struct TmuxStrategy;

impl TmuxStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TmuxStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SshSessionStrategy for TmuxStrategy {
    async fn create_session(
        &self,
        client: &SshClient,
        config: &RuntimeCreateConfig,
    ) -> Result<RuntimeHandle, SshError> {
        let name = shell::escape(&config.session_name);
        let command = shell::escape(&config.launch_command);
        let cwd = shell::escape(&config.cwd);

        let env_exports = build_env_exports(&config.env);

        let full_command =
            format!("cd {cwd} && {env_exports}tmux new-session -d -s {name} {command}");

        debug!(session_name = %config.session_name, "creating tmux session");

        let output = client.exec(&full_command).await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: full_command,
                    message: format!("tmux new-session failed (exit {}): {}", code, output.stderr),
                });
            }
        }

        let mut data = HashMap::new();
        data.insert(
            "runtime_type".to_string(),
            serde_json::Value::String("tmux".to_string()),
        );

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
        let cmd = format!("tmux kill-session -t {name}");

        debug!(session_name = %handle.runtime_name, "destroying tmux session");

        let output = client.exec(&cmd).await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: cmd,
                    message: format!(
                        "tmux kill-session failed (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
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

        if message.len() > SEND_KEYS_MAX_LEN {
            send_via_buffer(client, name, message).await
        } else {
            send_via_keys(client, name, message).await
        }
    }

    async fn get_output(
        &self,
        client: &SshClient,
        handle: &RuntimeHandle,
        lines: u32,
    ) -> Result<String, SshError> {
        let name = shell::escape(&handle.runtime_name);
        let cmd = format!("tmux capture-pane -t {name} -p -S -{lines}");

        let output = client.exec(&cmd).await?;

        if let Some(code) = output.exit_code {
            if code != 0 {
                return Err(SshError::Execution {
                    command: cmd,
                    message: format!(
                        "tmux capture-pane failed (exit {}): {}",
                        code, output.stderr
                    ),
                });
            }
        }

        Ok(output.stdout)
    }

    async fn is_alive(&self, client: &SshClient, handle: &RuntimeHandle) -> Result<bool, SshError> {
        let name = shell::escape(&handle.runtime_name);
        let cmd = format!("tmux has-session -t {name}");

        let output = client.exec(&cmd).await?;

        Ok(output.exit_code == Some(0))
    }
}

async fn send_via_keys(client: &SshClient, name: &str, message: &str) -> Result<(), SshError> {
    let escaped_name = shell::escape(name);
    let escaped_msg = shell::escape(message);
    let cmd = format!("tmux send-keys -t {escaped_name} {escaped_msg} Enter");

    debug!(session_name = %name, len = message.len(), "sending via send-keys");

    let output = client.exec(&cmd).await?;

    if let Some(code) = output.exit_code {
        if code != 0 {
            return Err(SshError::Execution {
                command: cmd,
                message: format!("tmux send-keys failed (exit {}): {}", code, output.stderr),
            });
        }
    }

    Ok(())
}

async fn send_via_buffer(client: &SshClient, name: &str, message: &str) -> Result<(), SshError> {
    let escaped_name = shell::escape(name);
    let escaped_msg = shell::escape(message);

    debug!(session_name = %name, len = message.len(), "sending via load-buffer");

    let mktemp_output = client.exec("mktemp /tmp/ennio-tmux-buf.XXXXXXXXXX").await?;
    let tmp_file = mktemp_output.stdout.trim();
    if tmp_file.is_empty() {
        return Err(SshError::Execution {
            command: "mktemp".to_owned(),
            message: "mktemp returned empty path".to_owned(),
        });
    }
    let escaped_tmp = shell::escape(tmp_file);

    let write_cmd = format!("printf '%s' {escaped_msg} > {escaped_tmp}");
    let write_output = client.exec(&write_cmd).await?;
    if let Some(code) = write_output.exit_code {
        if code != 0 {
            if let Err(e) = client.exec(&format!("rm -f {escaped_tmp}")).await {
                tracing::debug!("failed to clean up temp file: {e}");
            }
            return Err(SshError::Execution {
                command: write_cmd,
                message: format!(
                    "writing temp buffer failed (exit {}): {}",
                    code, write_output.stderr
                ),
            });
        }
    }

    let load_cmd = format!(
        "tmux load-buffer {escaped_tmp} && tmux paste-buffer -t {escaped_name} && tmux send-keys -t {escaped_name} Enter; rm -f {escaped_tmp}"
    );
    let load_output = client.exec(&load_cmd).await?;
    if let Some(code) = load_output.exit_code {
        if code != 0 {
            return Err(SshError::Execution {
                command: load_cmd,
                message: format!(
                    "tmux load-buffer/paste failed (exit {}): {}",
                    code, load_output.stderr
                ),
            });
        }
    }

    Ok(())
}
