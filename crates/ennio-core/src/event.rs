use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::id::{EventId, ProjectId, SessionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    SessionSpawned,
    SessionWorking,
    SessionExited,
    SessionKilled,
    SessionRestored,
    SessionCleaned,
    StatusChanged,
    ActivityChanged,
    PrCreated,
    PrUpdated,
    PrMerged,
    PrClosed,
    CiPassing,
    CiFailing,
    CiFixSent,
    CiFixFailed,
    ReviewPending,
    ReviewApproved,
    ReviewChangesRequested,
    ReviewCommentsSent,
    MergeReady,
    MergeConflicts,
    MergeCompleted,
    ReactionTriggered,
    ReactionEscalated,
    AllComplete,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventPriority {
    Info,
    Action,
    Urgent,
    Critical,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Info
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorEvent {
    pub id: EventId,
    pub event_type: EventType,
    pub priority: EventPriority,
    pub session_id: SessionId,
    pub project_id: ProjectId,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(EventPriority::Info, EventPriority::Action, true)]
    #[case(EventPriority::Action, EventPriority::Urgent, true)]
    #[case(EventPriority::Urgent, EventPriority::Critical, true)]
    #[case(EventPriority::Critical, EventPriority::Info, false)]
    fn priority_ordering(#[case] a: EventPriority, #[case] b: EventPriority, #[case] a_lt_b: bool) {
        assert_eq!(a < b, a_lt_b);
    }

    #[rstest]
    #[case(EventType::SessionSpawned, "session_spawned")]
    #[case(EventType::PrCreated, "pr_created")]
    #[case(EventType::AllComplete, "all_complete")]
    fn event_type_serialization(#[case] event: EventType, #[case] expected: &str) {
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, format!("\"{expected}\""));

        let deser: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(deser, event);
    }
}
