use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::EnnioError;
use crate::id::SessionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCreateConfig {
    pub session_id: SessionId,
    pub launch_command: String,
    pub env: HashMap<String, String>,
    pub cwd: String,
    pub session_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHandle {
    pub id: String,
    pub runtime_name: String,
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeMetrics {
    pub uptime: Duration,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachInfo {
    pub command: String,
    pub url: Option<String>,
    pub instructions: Option<String>,
}

#[async_trait]
pub trait Runtime: Send + Sync {
    fn name(&self) -> &str;

    async fn create(&self, config: &RuntimeCreateConfig) -> Result<RuntimeHandle, EnnioError>;

    async fn destroy(&self, handle: &RuntimeHandle) -> Result<(), EnnioError>;

    async fn send_message(&self, handle: &RuntimeHandle, message: &str) -> Result<(), EnnioError>;

    async fn get_output(&self, handle: &RuntimeHandle, lines: u32) -> Result<String, EnnioError>;

    async fn is_alive(&self, handle: &RuntimeHandle) -> Result<bool, EnnioError>;

    async fn get_metrics(&self, handle: &RuntimeHandle) -> Result<RuntimeMetrics, EnnioError>;

    async fn get_attach_info(&self, handle: &RuntimeHandle) -> Result<AttachInfo, EnnioError>;
}
