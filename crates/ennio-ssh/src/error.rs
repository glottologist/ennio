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
    fn connection_error_displays_host() {
        let err = SshError::Connection {
            host: "example.com".to_string(),
            message: "refused".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("example.com"));
        assert!(msg.contains("refused"));
    }

    #[test]
    fn timeout_error_displays_duration() {
        let err = SshError::Timeout {
            duration: Duration::from_secs(30),
        };
        let msg = err.to_string();
        assert!(msg.contains("30s"));
    }

    #[test]
    fn key_load_error_displays_path() {
        let err = SshError::KeyLoad {
            path: PathBuf::from("/home/user/.ssh/id_rsa"),
            message: "permission denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("id_rsa"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn channel_closed_error_displays() {
        let err = SshError::ChannelClosed;
        let msg = err.to_string();
        assert!(msg.contains("channel closed"));
    }

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
