use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::agent::AgentSessionInfo;
use crate::id::{ProjectId, SessionId};
use crate::runtime::RuntimeHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Spawning,
    Working,
    PrOpen,
    PrDraft,
    CiPassing,
    CiFailed,
    CiFixSent,
    CiFixFailed,
    ReviewPending,
    ChangesRequested,
    Approved,
    MergeConflicts,
    Merged,
    Done,
    Exited,
    Killed,
}

impl SessionStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Merged | Self::Done | Self::Exited | Self::Killed
        )
    }

    pub fn is_restorable(self) -> bool {
        matches!(self, Self::Exited)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    Active,
    Ready,
    Idle,
    WaitingInput,
    Blocked,
    Exited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityDetection {
    pub state: ActivityState,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub status: SessionStatus,
    pub activity: Option<ActivityState>,
    pub branch: Option<String>,
    pub issue_id: Option<String>,
    pub workspace_path: Option<PathBuf>,
    pub runtime_handle: Option<RuntimeHandle>,
    pub agent_info: Option<AgentSessionInfo>,
    pub agent_name: Option<String>,
    pub pr_url: Option<String>,
    pub pr_number: Option<i32>,
    pub tmux_name: Option<String>,
    pub config_hash: String,
    pub role: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub restored_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(SessionStatus::Spawning, false)]
    #[case(SessionStatus::Working, false)]
    #[case(SessionStatus::PrOpen, false)]
    #[case(SessionStatus::PrDraft, false)]
    #[case(SessionStatus::CiPassing, false)]
    #[case(SessionStatus::CiFailed, false)]
    #[case(SessionStatus::CiFixSent, false)]
    #[case(SessionStatus::CiFixFailed, false)]
    #[case(SessionStatus::ReviewPending, false)]
    #[case(SessionStatus::ChangesRequested, false)]
    #[case(SessionStatus::Approved, false)]
    #[case(SessionStatus::MergeConflicts, false)]
    #[case(SessionStatus::Merged, true)]
    #[case(SessionStatus::Done, true)]
    #[case(SessionStatus::Exited, true)]
    #[case(SessionStatus::Killed, true)]
    fn status_is_terminal(#[case] status: SessionStatus, #[case] expected: bool) {
        assert_eq!(status.is_terminal(), expected);
    }

    #[rstest]
    #[case(SessionStatus::Exited, true)]
    #[case(SessionStatus::Working, false)]
    #[case(SessionStatus::Killed, false)]
    #[case(SessionStatus::Merged, false)]
    fn status_is_restorable(#[case] status: SessionStatus, #[case] expected: bool) {
        assert_eq!(status.is_restorable(), expected);
    }
}
