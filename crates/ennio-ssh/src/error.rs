use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("SSH connection to {host} failed: {message}")]
    Connection { host: String, message: String },

    #[error("SSH authentication failed: {message}")]
    Authentication { message: String },

    #[error("SSH command execution failed for '{command}': {message}")]
    Execution { command: String, message: String },

    #[error("SSH channel closed unexpectedly")]
    ChannelClosed,

    #[error("SSH operation timed out after {duration:?}")]
    Timeout { duration: Duration },

    #[error("failed to load SSH key from {path}: {message}")]
    KeyLoad { path: PathBuf, message: String },

    #[error("SSH host key rejected for {host}: {message}")]
    HostKeyRejected { host: String, message: String },

    #[error("SSH host key changed for {host} (known_hosts line {line}) — possible MITM attack")]
    HostKeyChanged { host: String, line: usize },

    #[error("failed to read known_hosts: {message}")]
    KnownHostsRead { message: String },

    #[error("failed to write known_hosts: {message}")]
    KnownHostsWrite { message: String },

    #[error("SSH tunnel error: {message}")]
    Tunnel { message: String },

    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

impl From<SshError> for ennio_core::error::EnnioError {
    fn from(err: SshError) -> Self {
        Self::Ssh {
            message: err.to_string(),
        }
    }
}

impl From<russh::Error> for SshError {
    fn from(err: russh::Error) -> Self {
        Self::Connection {
            host: String::new(),
            message: err.to_string(),
        }
    }
}

impl From<russh_keys::Error> for SshError {
    fn from(err: russh_keys::Error) -> Self {
        Self::Authentication {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_to_ennio_error() {
        let err = SshError::Authentication {
            message: "bad key".to_string(),
        };
        let ennio_err: ennio_core::error::EnnioError = err.into();
        let msg = ennio_err.to_string();
        assert!(msg.contains("bad key"));
    }
}
