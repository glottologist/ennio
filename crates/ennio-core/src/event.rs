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
    NodeConnected,
    NodeDisconnected,
    NodeLaunched,
    NodeHealthCheck,
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
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn event_type_serde_roundtrips(
            event in prop::sample::select(vec![
                EventType::SessionSpawned,
                EventType::SessionWorking,
                EventType::SessionExited,
                EventType::SessionKilled,
                EventType::SessionRestored,
                EventType::SessionCleaned,
                EventType::StatusChanged,
                EventType::ActivityChanged,
                EventType::PrCreated,
                EventType::PrUpdated,
                EventType::PrMerged,
                EventType::PrClosed,
                EventType::CiPassing,
                EventType::CiFailing,
                EventType::CiFixSent,
                EventType::CiFixFailed,
                EventType::ReviewPending,
                EventType::ReviewApproved,
                EventType::ReviewChangesRequested,
                EventType::ReviewCommentsSent,
                EventType::MergeReady,
                EventType::MergeConflicts,
                EventType::MergeCompleted,
                EventType::ReactionTriggered,
                EventType::ReactionEscalated,
                EventType::AllComplete,
                EventType::NodeConnected,
                EventType::NodeDisconnected,
                EventType::NodeLaunched,
                EventType::NodeHealthCheck,
            ])
        ) {
            let json = serde_json::to_string(&event).unwrap();
            let deser: EventType = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(deser, event);
        }
    }
}
