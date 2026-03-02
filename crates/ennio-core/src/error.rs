use std::path::PathBuf;

use crate::id::{ProjectId, SessionId};

#[derive(Debug, thiserror::Error)]
pub enum EnnioError {
    #[error("config error: {message}")]
    Config { message: String },

    #[error("database error: {message}")]
    Database { message: String },

    #[error("SSH error: {message}")]
    Ssh { message: String },

    #[error("NATS error: {message}")]
    Nats { message: String },

    #[error("plugin error in {plugin}: {message}")]
    Plugin { plugin: String, message: String },

    #[error("session error for {session_id}: {message}")]
    Session {
        session_id: SessionId,
        message: String,
    },

    #[error("workspace error: {message}")]
    Workspace { message: String },

    #[error("runtime error: {message}")]
    Runtime { message: String },

    #[error("agent error: {message}")]
    Agent { message: String },

    #[error("tracker error: {message}")]
    Tracker { message: String },

    #[error("SCM error: {message}")]
    Scm { message: String },

    #[error("notifier error: {message}")]
    Notifier { message: String },

    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: Option<PathBuf>,
        source: std::io::Error,
    },

    #[error("serialization error: {message}")]
    Serialization { message: String },

    #[error("budget exceeded for project {project_id}: {message}")]
    Budget {
        project_id: ProjectId,
        message: String,
    },

    #[error("ledger error: {message}")]
    Ledger { message: String },

    #[error("invalid ID: {value} ({reason})")]
    InvalidId { value: String, reason: String },

    #[error("{entity} not found: {id}")]
    NotFound { entity: String, id: String },

    #[error("{entity} already exists: {id}")]
    AlreadyExists { entity: String, id: String },

    #[error("operation timed out after {duration:?}: {message}")]
    Timeout {
        duration: std::time::Duration,
        message: String,
    },

    #[error("node error on {host}: {message}")]
    Node { host: String, message: String },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl From<std::io::Error> for EnnioError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            path: None,
            source: err,
        }
    }
}

impl From<serde_json::Error> for EnnioError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
        }
    }
}
