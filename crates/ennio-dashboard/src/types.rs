use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub activity: Option<String>,
    pub branch: Option<String>,
    pub pr_url: Option<String>,
    pub pr_number: Option<i32>,
    pub agent_name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

impl SessionSummary {
    #[must_use]
    pub fn needs_attention(&self) -> bool {
        matches!(
            self.status.as_str(),
            "ci_failed" | "ci_fix_failed" | "changes_requested" | "merge_conflicts"
        )
    }

    #[must_use]
    pub fn attention_reason(&self) -> Option<&'static str> {
        match self.status.as_str() {
            "ci_failed" => Some("CI failed"),
            "ci_fix_failed" => Some("CI fix attempt failed"),
            "changes_requested" => Some("Changes requested by reviewer"),
            "merge_conflicts" => Some("Merge conflicts detected"),
            _ => None,
        }
    }

    #[must_use]
    pub fn status_label(&self) -> &'static str {
        match self.status.as_str() {
            "spawning" => "Spawning",
            "working" => "Working",
            "pr_open" => "PR Open",
            "pr_draft" => "PR Draft",
            "ci_passing" => "CI Passing",
            "ci_failed" => "CI Failed",
            "ci_fix_sent" => "CI Fix Sent",
            "ci_fix_failed" => "CI Fix Failed",
            "review_pending" => "Review Pending",
            "changes_requested" => "Changes Requested",
            "approved" => "Approved",
            "merge_conflicts" => "Merge Conflicts",
            "merged" => "Merged",
            "done" => "Done",
            "exited" => "Exited",
            "killed" => "Killed",
            _ => "Unknown",
        }
    }

    #[must_use]
    pub fn status_color(&self) -> &'static str {
        match self.status.as_str() {
            "spawning" | "working" => "#3b82f6",
            "pr_open" | "pr_draft" => "#8b5cf6",
            "ci_passing" | "approved" => "#22c55e",
            "ci_failed" | "ci_fix_failed" => "#ef4444",
            "ci_fix_sent" | "review_pending" => "#f59e0b",
            "changes_requested" | "merge_conflicts" => "#f97316",
            "merged" | "done" => "#6b7280",
            "exited" | "killed" => "#374151",
            _ => "#9ca3af",
        }
    }

    #[must_use]
    pub fn activity_label(&self) -> &'static str {
        match self.activity.as_deref() {
            Some("active") => "Active",
            Some("ready") => "Ready",
            Some("idle") => "Idle",
            Some("waiting_input") => "Waiting Input",
            Some("blocked") => "Blocked",
            Some("exited") => "Exited",
            _ => "Unknown",
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            "merged" | "done" | "exited" | "killed"
        )
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PrStatusEntry {
    pub session_id: String,
    pub pr_number: Option<i32>,
    pub pr_url: Option<String>,
    pub branch: Option<String>,
    pub status: String,
    pub ci_status: Option<String>,
    pub review_status: Option<String>,
}

impl PrStatusEntry {
    #[must_use]
    pub fn ci_label(&self) -> &'static str {
        match self.ci_status.as_deref() {
            Some("passing") => "Passing",
            Some("failing") => "Failing",
            Some("pending") => "Pending",
            Some("running") => "Running",
            Some("cancelled") => "Cancelled",
            Some("skipped") => "Skipped",
            _ => "N/A",
        }
    }

    #[must_use]
    pub fn ci_color(&self) -> &'static str {
        match self.ci_status.as_deref() {
            Some("passing") => "#22c55e",
            Some("failing") => "#ef4444",
            Some("pending") | Some("running") => "#f59e0b",
            _ => "#9ca3af",
        }
    }

    #[must_use]
    pub fn review_label(&self) -> &'static str {
        match self.review_status.as_deref() {
            Some("approved") => "Approved",
            Some("changes_requested") => "Changes Requested",
            Some("pending") => "Pending",
            Some("dismissed") => "Dismissed",
            _ => "N/A",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DashboardData {
    pub sessions: Vec<SessionSummary>,
    pub pr_statuses: Vec<PrStatusEntry>,
}

impl DashboardData {
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.sessions.iter().filter(|s| !s.is_terminal()).count()
    }

    #[must_use]
    pub fn attention_count(&self) -> usize {
        self.sessions.iter().filter(|s| s.needs_attention()).count()
    }

    #[must_use]
    pub fn terminal_count(&self) -> usize {
        self.sessions.iter().filter(|s| s.is_terminal()).count()
    }

    #[must_use]
    pub fn attention_sessions(&self) -> Vec<&SessionSummary> {
        self.sessions
            .iter()
            .filter(|s| s.needs_attention())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    fn make_session(status: &str, activity: Option<&str>) -> SessionSummary {
        SessionSummary {
            id: "s1".to_owned(),
            project_id: "p1".to_owned(),
            status: status.to_owned(),
            activity: activity.map(str::to_owned),
            branch: None,
            pr_url: None,
            pr_number: None,
            agent_name: None,
            created_at: None,
            last_activity_at: None,
        }
    }

    #[rstest]
    #[case("spawning", "Spawning")]
    #[case("working", "Working")]
    #[case("pr_open", "PR Open")]
    #[case("pr_draft", "PR Draft")]
    #[case("ci_passing", "CI Passing")]
    #[case("ci_failed", "CI Failed")]
    #[case("ci_fix_sent", "CI Fix Sent")]
    #[case("ci_fix_failed", "CI Fix Failed")]
    #[case("review_pending", "Review Pending")]
    #[case("changes_requested", "Changes Requested")]
    #[case("approved", "Approved")]
    #[case("merge_conflicts", "Merge Conflicts")]
    #[case("merged", "Merged")]
    #[case("done", "Done")]
    #[case("exited", "Exited")]
    #[case("killed", "Killed")]
    #[case("unknown_status", "Unknown")]
    fn status_label_exhaustive(#[case] status: &str, #[case] expected: &str) {
        let s = make_session(status, None);
        assert_eq!(s.status_label(), expected);
    }

    #[rstest]
    #[case("spawning", "#3b82f6")]
    #[case("working", "#3b82f6")]
    #[case("pr_open", "#8b5cf6")]
    #[case("pr_draft", "#8b5cf6")]
    #[case("ci_passing", "#22c55e")]
    #[case("approved", "#22c55e")]
    #[case("ci_failed", "#ef4444")]
    #[case("ci_fix_failed", "#ef4444")]
    #[case("ci_fix_sent", "#f59e0b")]
    #[case("review_pending", "#f59e0b")]
    #[case("changes_requested", "#f97316")]
    #[case("merge_conflicts", "#f97316")]
    #[case("merged", "#6b7280")]
    #[case("done", "#6b7280")]
    #[case("exited", "#374151")]
    #[case("killed", "#374151")]
    #[case("unknown_status", "#9ca3af")]
    fn status_color_exhaustive(#[case] status: &str, #[case] expected: &str) {
        let s = make_session(status, None);
        assert_eq!(s.status_color(), expected);
    }

    #[rstest]
    #[case(Some("active"), "Active")]
    #[case(Some("ready"), "Ready")]
    #[case(Some("idle"), "Idle")]
    #[case(Some("waiting_input"), "Waiting Input")]
    #[case(Some("blocked"), "Blocked")]
    #[case(Some("exited"), "Exited")]
    #[case(None, "Unknown")]
    #[case(Some("other"), "Unknown")]
    fn activity_label_exhaustive(#[case] activity: Option<&str>, #[case] expected: &str) {
        let s = make_session("working", activity);
        assert_eq!(s.activity_label(), expected);
    }

    const ATTENTION_STATUSES: &[&str] = &[
        "ci_failed",
        "ci_fix_failed",
        "changes_requested",
        "merge_conflicts",
    ];
    const TERMINAL_STATUSES: &[&str] = &["merged", "done", "exited", "killed"];

    proptest! {
        #[test]
        fn needs_attention_consistent_with_attention_reason(
            status in "[a-z_]{1,20}"
        ) {
            let s = make_session(&status, None);
            let needs = s.needs_attention();
            let reason = s.attention_reason();
            prop_assert_eq!(needs, reason.is_some());
        }

        #[test]
        fn is_terminal_consistent_with_known_statuses(
            status in prop::sample::select(vec![
                "spawning", "working", "pr_open", "pr_draft",
                "ci_passing", "ci_failed", "ci_fix_sent", "ci_fix_failed",
                "review_pending", "changes_requested", "approved",
                "merge_conflicts", "merged", "done", "exited", "killed",
            ])
        ) {
            let s = make_session(&status, None);
            let expected = TERMINAL_STATUSES.contains(&&*status);
            prop_assert_eq!(s.is_terminal(), expected);
        }

        #[test]
        fn active_plus_terminal_equals_total(
            statuses in prop::collection::vec(
                prop::sample::select(vec![
                    "spawning", "working", "pr_open", "ci_failed",
                    "merged", "done", "exited", "killed",
                ]),
                0..20,
            )
        ) {
            let sessions: Vec<SessionSummary> = statuses
                .iter()
                .enumerate()
                .map(|(i, s)| SessionSummary {
                    id: i.to_string(),
                    project_id: "p1".to_owned(),
                    status: s.to_string(),
                    activity: None,
                    branch: None,
                    pr_url: None,
                    pr_number: None,
                    agent_name: None,
                    created_at: None,
                    last_activity_at: None,
                })
                .collect();
            let data = DashboardData {
                sessions,
                pr_statuses: vec![],
            };
            prop_assert_eq!(
                data.active_count() + data.terminal_count(),
                data.sessions.len()
            );
        }

        #[test]
        fn attention_sessions_subset_of_all(
            statuses in prop::collection::vec(
                prop::sample::select(vec![
                    "spawning", "working", "ci_failed",
                    "changes_requested", "merged", "done",
                ]),
                0..15,
            )
        ) {
            let sessions: Vec<SessionSummary> = statuses
                .iter()
                .enumerate()
                .map(|(i, s)| SessionSummary {
                    id: i.to_string(),
                    project_id: "p1".to_owned(),
                    status: s.to_string(),
                    activity: None,
                    branch: None,
                    pr_url: None,
                    pr_number: None,
                    agent_name: None,
                    created_at: None,
                    last_activity_at: None,
                })
                .collect();
            let data = DashboardData {
                sessions,
                pr_statuses: vec![],
            };
            prop_assert_eq!(data.attention_count(), data.attention_sessions().len());
            prop_assert!(data.attention_count() <= data.sessions.len());
        }
    }
}
