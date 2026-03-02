pub mod client;
pub mod config;
pub mod error;
pub mod shell;
pub mod strategy;

pub use client::{ExecOutput, SshClient};
pub use config::{HostKeyPolicy, SshAuth, SshConfig, SshStrategy};
pub use error::SshError;
pub use strategy::{
    RemoteControlStrategy, SshSessionStrategy, TmateStrategy, TmuxStrategy, create_strategy,
};
