use std::path::PathBuf;
use std::sync::Arc;

use ennio_core::config::{HostKeyPolicyConfig, SshAuthConfig, SshConnectionConfig};
use russh::keys::ssh_key;
use secrecy::ExposeSecret;
use tokio::sync::Mutex;

use crate::error::SshError;

#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<u32>,
}

struct ClientHandler {
    host_key_policy: HostKeyPolicyConfig,
    host: String,
    port: u16,
    known_hosts_path: Option<PathBuf>,
}

impl ClientHandler {
    fn check_known_hosts(&self, server_public_key: &ssh_key::PublicKey) -> Result<bool, SshError> {
        let result = match &self.known_hosts_path {
            Some(path) => russh_keys::known_hosts::check_known_hosts_path(
                &self.host,
                self.port,
                server_public_key,
                path,
            ),
            None => russh_keys::check_known_hosts(&self.host, self.port, server_public_key),
        };
        match result {
            Ok(matched) => Ok(matched),
            Err(russh_keys::Error::KeyChanged { line }) => Err(SshError::HostKeyChanged {
                host: self.host.clone(), // clone: host needed for error context
                line,
            }),
            Err(e) => Err(SshError::KnownHostsRead {
                message: e.to_string(),
            }),
        }
    }

    fn persist_host_key(&self, server_public_key: &ssh_key::PublicKey) -> Result<(), SshError> {
        let result = match &self.known_hosts_path {
            Some(path) => russh_keys::known_hosts::learn_known_hosts_path(
                &self.host,
                self.port,
                server_public_key,
                path,
            ),
            None => {
                russh_keys::known_hosts::learn_known_hosts(&self.host, self.port, server_public_key)
            }
        };
        result.map_err(|e| SshError::KnownHostsWrite {
            message: e.to_string(),
        })
    }

    fn accept_new_key(&self, server_public_key: &ssh_key::PublicKey) -> Result<bool, SshError> {
        match self.check_known_hosts(server_public_key)? {
            true => Ok(true),
            false => {
                tracing::info!(host = %self.host, "persisting new host key");
                self.persist_host_key(server_public_key)?;
                Ok(true)
            }
        }
    }

    fn verify_known_key(&self, server_public_key: &ssh_key::PublicKey) -> Result<bool, SshError> {
        match self.check_known_hosts(server_public_key)? {
            true => Ok(true),
            false => Err(SshError::HostKeyRejected {
                host: self.host.clone(), // clone: host needed for error context
                message: "host key not found in known_hosts".into(),
            }),
        }
    }
}

#[async_trait::async_trait]
impl russh::client::Handler for ClientHandler {
    type Error = SshError;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match self.host_key_policy {
            HostKeyPolicyConfig::AcceptAll => Ok(true),
            HostKeyPolicyConfig::AcceptNew => self.accept_new_key(server_public_key),
            HostKeyPolicyConfig::Strict => self.verify_known_key(server_public_key),
        }
    }
}

pub struct SshClient {
    handle: Arc<Mutex<russh::client::Handle<ClientHandler>>>,
    config: SshConnectionConfig,
}

impl Clone for SshClient {
    fn clone(&self) -> Self {
        Self {
            handle: Arc::clone(&self.handle),
            config: self.config.clone(), // clone: SshConnectionConfig is pure data needed for reconnect
        }
    }
}

impl SshClient {
    pub async fn connect(config: &SshConnectionConfig) -> Result<Self, SshError> {
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
    config: &SshConnectionConfig,
) -> Result<russh::client::Handle<ClientHandler>, SshError> {
    let ssh_config = russh::client::Config {
        inactivity_timeout: Some(config.connection_timeout),
        keepalive_interval: config.keepalive_interval,
        ..Default::default()
    };

    let handler = ClientHandler {
        host_key_policy: config.host_key_policy,
        host: config.host.clone(), // clone: host needed for error reporting in handler
        port: config.port,
        known_hosts_path: config.known_hosts_path.clone(), // clone: path needed in handler for known_hosts lookup
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
    config: &SshConnectionConfig,
) -> Result<(), SshError> {
    let authenticated = match &config.auth {
        SshAuthConfig::Key { path, passphrase } => {
            let key_pair = load_key(path, passphrase.as_ref().map(|s| s.expose_secret())).await?;
            handle
                .authenticate_publickey(&config.username, Arc::new(key_pair))
                .await
                .map_err(|e| SshError::Authentication {
                    message: format!("public key auth failed: {e}"),
                })?
        }
        SshAuthConfig::Password { password } => handle
            .authenticate_password(&config.username, password.expose_secret())
            .await
            .map_err(|e| SshError::Authentication {
                message: format!("password auth failed: {e}"),
            })?,
        SshAuthConfig::Agent => {
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

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    const TEST_KEY_A: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILM+rvN+ot98qgEN796jTiQfZfG1KaT0PtFDJ/XFSqti";
    const TEST_KEY_B: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGkRGFKhIT0dEsn5wPnKw+JaoJBZ5cFAMOtnPKMi0Vzt";

    fn parse_pubkey(openssh: &str) -> ssh_key::PublicKey {
        ssh_key::PublicKey::from_openssh(openssh).expect("valid test key")
    }

    fn handler_with_path(policy: HostKeyPolicyConfig, path: &std::path::Path) -> ClientHandler {
        ClientHandler {
            host_key_policy: policy,
            host: "test.example.com".to_string(),
            port: 22,
            known_hosts_path: Some(path.to_path_buf()),
        }
    }

    #[test]
    fn host_key_rejected_error_contains_host() {
        let err = SshError::HostKeyRejected {
            host: "test.example.com".to_string(),
            message: "strict policy rejects all unverified keys".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("test.example.com"));
        assert!(msg.contains("strict policy"));
    }

    #[test]
    fn host_key_changed_error_contains_line() {
        let err = SshError::HostKeyChanged {
            host: "evil.example.com".to_string(),
            line: 42,
        };
        let msg = err.to_string();
        assert!(msg.contains("evil.example.com"));
        assert!(msg.contains("42"));
        assert!(msg.contains("MITM"));
    }

    #[test]
    fn accept_new_persists_key_to_known_hosts() {
        let file = NamedTempFile::new().unwrap();
        let handler = handler_with_path(HostKeyPolicyConfig::AcceptNew, file.path());
        let key = parse_pubkey(TEST_KEY_A);

        let result = handler.accept_new_key(&key);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let contents = std::fs::read_to_string(file.path()).unwrap();
        assert!(contents.contains("test.example.com"));
    }

    #[test]
    fn accept_new_accepts_already_known_key() {
        let file = NamedTempFile::new().unwrap();
        let handler = handler_with_path(HostKeyPolicyConfig::AcceptNew, file.path());
        let key = parse_pubkey(TEST_KEY_A);

        handler.persist_host_key(&key).unwrap();

        let result = handler.accept_new_key(&key);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn accept_new_rejects_changed_key() {
        let file = NamedTempFile::new().unwrap();
        let handler = handler_with_path(HostKeyPolicyConfig::AcceptNew, file.path());
        let key_a = parse_pubkey(TEST_KEY_A);
        let key_b = parse_pubkey(TEST_KEY_B);

        handler.persist_host_key(&key_a).unwrap();

        let result = handler.accept_new_key(&key_b);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SshError::HostKeyChanged { .. }),
            "expected HostKeyChanged, got: {err}"
        );
    }

    #[test]
    fn strict_accepts_known_key() {
        let file = NamedTempFile::new().unwrap();
        let handler = handler_with_path(HostKeyPolicyConfig::Strict, file.path());
        let key = parse_pubkey(TEST_KEY_A);

        handler.persist_host_key(&key).unwrap();

        let result = handler.verify_known_key(&key);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn strict_rejects_unknown_key() {
        let file = NamedTempFile::new().unwrap();
        let handler = handler_with_path(HostKeyPolicyConfig::Strict, file.path());
        let key = parse_pubkey(TEST_KEY_A);

        let result = handler.verify_known_key(&key);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SshError::HostKeyRejected { .. }),
            "expected HostKeyRejected, got: {err}"
        );
    }

    #[test]
    fn strict_rejects_changed_key() {
        let file = NamedTempFile::new().unwrap();
        let handler = handler_with_path(HostKeyPolicyConfig::Strict, file.path());
        let key_a = parse_pubkey(TEST_KEY_A);
        let key_b = parse_pubkey(TEST_KEY_B);

        handler.persist_host_key(&key_a).unwrap();

        let result = handler.verify_known_key(&key_b);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, SshError::HostKeyChanged { .. }),
            "expected HostKeyChanged, got: {err}"
        );
    }

    #[test]
    fn accept_all_ignores_known_hosts() {
        let file = NamedTempFile::new().unwrap();
        let handler = handler_with_path(HostKeyPolicyConfig::AcceptAll, file.path());
        let key_a = parse_pubkey(TEST_KEY_A);
        let key_b = parse_pubkey(TEST_KEY_B);

        handler.persist_host_key(&key_a).unwrap();

        // check_known_hosts still detects key mismatch (policy-agnostic)
        let result = handler.check_known_hosts(&key_b);
        assert!(matches!(result, Err(SshError::HostKeyChanged { .. })));

        let contents = std::fs::read_to_string(file.path()).unwrap();
        assert!(!contents.is_empty());
    }
}
