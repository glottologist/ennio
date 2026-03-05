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

    pub fn needs_attention(self) -> bool {
        matches!(
            self,
            Self::CiFailed | Self::CiFixFailed | Self::ChangesRequested | Self::MergeConflicts
        )
    }

    pub fn attention_reason(self) -> Option<&'static str> {
        match self {
            Self::CiFailed => Some("CI failed"),
            Self::CiFixFailed => Some("CI fix attempt failed"),
            Self::ChangesRequested => Some("Changes requested by reviewer"),
            Self::MergeConflicts => Some("Merge conflicts detected"),
            _ => None,
        }
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::Spawning => "Spawning",
            Self::Working => "Working",
            Self::PrOpen => "PR Open",
            Self::PrDraft => "PR Draft",
            Self::CiPassing => "CI Passing",
            Self::CiFailed => "CI Failed",
            Self::CiFixSent => "CI Fix Sent",
            Self::CiFixFailed => "CI Fix Failed",
            Self::ReviewPending => "Review Pending",
            Self::ChangesRequested => "Changes Requested",
            Self::Approved => "Approved",
            Self::MergeConflicts => "Merge Conflicts",
            Self::Merged => "Merged",
            Self::Done => "Done",
            Self::Exited => "Exited",
            Self::Killed => "Killed",
        }
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

    #[rstest]
    #[case(SessionStatus::CiFailed, true)]
    #[case(SessionStatus::CiFixFailed, true)]
    #[case(SessionStatus::ChangesRequested, true)]
    #[case(SessionStatus::MergeConflicts, true)]
    #[case(SessionStatus::Spawning, false)]
    #[case(SessionStatus::Working, false)]
    #[case(SessionStatus::Approved, false)]
    #[case(SessionStatus::Merged, false)]
    fn status_needs_attention(#[case] status: SessionStatus, #[case] expected: bool) {
        assert_eq!(status.needs_attention(), expected);
        assert_eq!(status.attention_reason().is_some(), expected);
    }

    #[rstest]
    #[case(SessionStatus::Spawning, "Spawning")]
    #[case(SessionStatus::Working, "Working")]
    #[case(SessionStatus::PrOpen, "PR Open")]
    #[case(SessionStatus::PrDraft, "PR Draft")]
    #[case(SessionStatus::CiPassing, "CI Passing")]
    #[case(SessionStatus::CiFailed, "CI Failed")]
    #[case(SessionStatus::CiFixSent, "CI Fix Sent")]
    #[case(SessionStatus::CiFixFailed, "CI Fix Failed")]
    #[case(SessionStatus::ReviewPending, "Review Pending")]
    #[case(SessionStatus::ChangesRequested, "Changes Requested")]
    #[case(SessionStatus::Approved, "Approved")]
    #[case(SessionStatus::MergeConflicts, "Merge Conflicts")]
    #[case(SessionStatus::Merged, "Merged")]
    #[case(SessionStatus::Done, "Done")]
    #[case(SessionStatus::Exited, "Exited")]
    #[case(SessionStatus::Killed, "Killed")]
    fn display_label_correct(#[case] status: SessionStatus, #[case] expected: &str) {
        assert_eq!(status.display_label(), expected);
    }
}
