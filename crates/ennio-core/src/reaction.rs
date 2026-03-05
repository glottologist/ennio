use std::time::Duration;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::event::EventPriority;
use crate::serde_helpers::option_duration_secs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReactionAction {
    SendToAgent,
    Notify,
    AutoMerge,
}

impl Default for ReactionAction {
    fn default() -> Self {
        Self::Notify
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionConfig {
    pub enabled: bool,
    pub action: ReactionAction,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub priority: EventPriority,
    #[serde(default)]
    #[serde(with = "option_duration_secs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalate_after: Option<Duration>,
    #[serde(default)]
    #[serde(with = "option_duration_secs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Duration>,
    #[serde(default)]
    pub retries: u32,
    #[serde(default)]
    pub include_summary: bool,
}

impl Default for ReactionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            action: ReactionAction::default(),
            message: None,
            priority: EventPriority::default(),
            escalate_after: None,
            threshold: None,
            retries: 0,
            include_summary: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionResult {
    pub success: bool,
    pub action_taken: ReactionAction,
    pub message: String,
    pub escalated: bool,
}
