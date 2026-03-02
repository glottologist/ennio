use std::sync::Arc;

use russh::keys::ssh_key;
use tokio::sync::Mutex;

use crate::config::{HostKeyPolicy, SshAuth, SshConfig};
use crate::error::SshError;

#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<u32>,
}

struct ClientHandler {
    host_key_policy: HostKeyPolicy,
}

#[async_trait::async_trait]
impl russh::client::Handler for ClientHandler {
    type Error = SshError;

    async fn check_server_key(
        &mut self,
        _server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match self.host_key_policy {
            HostKeyPolicy::AcceptAll => Ok(true),
            // TODO: implement known_hosts checking via russh_keys::known_hosts
            HostKeyPolicy::Strict | HostKeyPolicy::AcceptNew => Ok(true),
        }
    }
}

pub struct SshClient {
    handle: Arc<Mutex<russh::client::Handle<ClientHandler>>>,
    config: SshConfig,
}

impl Clone for SshClient {
    fn clone(&self) -> Self {
        Self {
            handle: Arc::clone(&self.handle),
            config: self.config.clone(), // clone: SshConfig is pure data needed for reconnect
        }
    }
}

impl SshClient {
    pub async fn connect(config: &SshConfig) -> Result<Self, SshError> {
        let handle = establish_connection(config).await?;
        Ok(Self {
            handle: Arc::new(Mutex::new(handle)),
            config: config.clone(), // clone: need owned copy for reconnect
        })
    }

    pub async fn exec(&self, command: &str) -> Result<ExecOutput, SshError> {
        let handle = self.handle.lock().await;
        let mut channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SshError::Execution {
                command: command.to_string(),
                message: format!("failed to open channel: {e}"),
            })?;

        channel
            .exec(true, command.as_bytes())
            .await
            .map_err(|e| SshError::Execution {
                command: command.to_string(),
                message: format!("exec failed: {e}"),
            })?;

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        let mut exit_code: Option<u32> = None;

        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    stdout_buf.extend_from_slice(data);
                }
                russh::ChannelMsg::ExtendedData { ref data, ext } => {
                    if ext == 1 {
                        stderr_buf.extend_from_slice(data);
                    }
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = Some(exit_status);
                }
                _ => {}
            }
        }

        let stdout = String::from_utf8_lossy(&stdout_buf).into_owned();
        let stderr = String::from_utf8_lossy(&stderr_buf).into_owned();

        Ok(ExecOutput {
            stdout,
            stderr,
            exit_code,
        })
    }

    pub async fn exec_detached(&self, command: &str) -> Result<(), SshError> {
        let nohup_cmd = format!("nohup {command} </dev/null >/dev/null 2>&1 &");
        let handle = self.handle.lock().await;
        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| SshError::Execution {
                command: command.to_string(),
                message: format!("failed to open channel: {e}"),
            })?;

        channel
            .exec(true, nohup_cmd.as_bytes())
            .await
            .map_err(|e| SshError::Execution {
                command: command.to_string(),
                message: format!("exec_detached failed: {e}"),
            })?;

        channel.eof().await.map_err(|e| SshError::Execution {
            command: command.to_string(),
            message: format!("failed to send eof: {e}"),
        })?;

        Ok(())
    }

    pub async fn forward_local_port(
        &self,
        local_port: u16,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<(), SshError> {
        let handle = self.handle.lock().await;
        handle
            .channel_open_direct_tcpip(
                remote_host,
                u32::from(remote_port),
                "127.0.0.1",
                u32::from(local_port),
            )
            .await
            .map_err(|e| SshError::Tunnel {
                message: format!(
                    "failed to open tunnel 127.0.0.1:{local_port} -> {remote_host}:{remote_port}: {e}"
                ),
            })?;

        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        let handle = self.handle.lock().await;
        !handle.is_closed()
    }

    pub async fn reconnect(&self) -> Result<(), SshError> {
        let new_handle = establish_connection(&self.config).await?;
        let mut handle = self.handle.lock().await;
        *handle = new_handle;
        Ok(())
    }
}

async fn establish_connection(
    config: &SshConfig,
) -> Result<russh::client::Handle<ClientHandler>, SshError> {
    let ssh_config = russh::client::Config {
        inactivity_timeout: Some(config.connection_timeout),
        keepalive_interval: config.keepalive_interval,
        ..Default::default()
    };

    let handler = ClientHandler {
        host_key_policy: config.host_key_policy,
    };
    let mut handle = tokio::time::timeout(
        config.connection_timeout,
        russh::client::connect(Arc::new(ssh_config), (&*config.host, config.port), handler),
    )
    .await
    .map_err(|_| SshError::Timeout {
        duration: config.connection_timeout,
    })?
    .map_err(|e| SshError::Connection {
        host: config.host.clone(), // clone: building error with host context
        message: e.to_string(),
    })?;

    authenticate(&mut handle, config).await?;

    Ok(handle)
}

async fn authenticate(
    handle: &mut russh::client::Handle<ClientHandler>,
    config: &SshConfig,
) -> Result<(), SshError> {
    let authenticated = match &config.auth {
        SshAuth::Key { path, passphrase } => {
            let key_pair = load_key(path, passphrase.as_deref()).await?;
            handle
                .authenticate_publickey(&config.username, Arc::new(key_pair))
                .await
                .map_err(|e| SshError::Authentication {
                    message: format!("public key auth failed: {e}"),
                })?
        }
        SshAuth::Password { password } => handle
            .authenticate_password(&config.username, password)
            .await
            .map_err(|e| SshError::Authentication {
                message: format!("password auth failed: {e}"),
            })?,
        SshAuth::Agent => {
            return Err(SshError::Authentication {
                message: "SSH agent authentication not yet implemented".to_string(),
            });
        }
    };

    if !authenticated {
        return Err(SshError::Authentication {
            message: "server rejected credentials".to_string(),
        });
    }

    Ok(())
}

async fn load_key(
    path: &std::path::Path,
    passphrase: Option<&str>,
) -> Result<ssh_key::PrivateKey, SshError> {
    let key_data = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| SshError::KeyLoad {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

    match passphrase {
        Some(phrase) => ssh_key::PrivateKey::from_openssh(key_data.as_bytes())
            .and_then(|k| k.decrypt(phrase.as_bytes()))
            .map_err(|e| SshError::KeyLoad {
                path: path.to_path_buf(),
                message: format!("failed to parse/decrypt key: {e}"),
            }),
        None => {
            ssh_key::PrivateKey::from_openssh(key_data.as_bytes()).map_err(|e| SshError::KeyLoad {
                path: path.to_path_buf(),
                message: format!("failed to parse key: {e}"),
            })
        }
    }
}

impl std::fmt::Debug for SshClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshClient")
            .field("host", &self.config.host)
            .field("port", &self.config.port)
            .field("username", &self.config.username)
            .finish_non_exhaustive()
    }
}
