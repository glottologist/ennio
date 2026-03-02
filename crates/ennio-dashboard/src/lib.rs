pub mod components;
pub mod types;

use dioxus::prelude::*;

use crate::components::{AttentionZone, PrTable, SessionList, StatusBar};
use crate::types::{DashboardData, PrStatusEntry, SessionSummary};

/// Root application component for the Ennio dashboard.
///
/// Renders a full-page dashboard with a status bar, attention zone,
/// session cards, and a PR status table.
#[component]
pub fn App() -> Element {
    let data = use_signal(sample_dashboard_data);

    let dashboard = data.read();

    rsx! {
        div {
            style: APP_STYLE,
            StatusBar {
                total: dashboard.sessions.len(),
                active: dashboard.active_count(),
                attention: dashboard.attention_count(),
                terminal: dashboard.terminal_count(),
            }
            AttentionZone {
                sessions: dashboard.sessions.clone(), // clone: owned Vec needed for child component props
            }
            PrTable {
                entries: dashboard.pr_statuses.clone(), // clone: owned Vec needed for child component props
            }
            SessionList {
                sessions: dashboard.sessions.clone(), // clone: owned Vec needed for child component props
            }
        }
    }
}

fn sample_dashboard_data() -> DashboardData {
    DashboardData {
        sessions: sample_sessions(),
        pr_statuses: sample_pr_statuses(),
    }
}

fn sample_sessions() -> Vec<SessionSummary> {
    vec![
        SessionSummary {
            id: "sess-001".into(),
            project_id: "proj-alpha".into(),
            status: "working".into(),
            activity: Some("active".into()),
            branch: Some("feat/add-auth".into()),
            pr_url: None,
            pr_number: None,
            agent_name: Some("claude".into()),
            created_at: None,
            last_activity_at: None,
        },
        SessionSummary {
            id: "sess-002".into(),
            project_id: "proj-alpha".into(),
            status: "ci_failed".into(),
            activity: Some("idle".into()),
            branch: Some("fix/db-timeout".into()),
            pr_url: Some("https://github.com/example/repo/pull/42".into()),
            pr_number: Some(42),
            agent_name: Some("claude".into()),
            created_at: None,
            last_activity_at: None,
        },
        SessionSummary {
            id: "sess-003".into(),
            project_id: "proj-beta".into(),
            status: "changes_requested".into(),
            activity: Some("waiting_input".into()),
            branch: Some("feat/new-api".into()),
            pr_url: Some("https://github.com/example/repo/pull/43".into()),
            pr_number: Some(43),
            agent_name: Some("codex".into()),
            created_at: None,
            last_activity_at: None,
        },
        SessionSummary {
            id: "sess-004".into(),
            project_id: "proj-gamma".into(),
            status: "approved".into(),
            activity: Some("ready".into()),
            branch: Some("feat/dashboard".into()),
            pr_url: Some("https://github.com/example/repo/pull/44".into()),
            pr_number: Some(44),
            agent_name: Some("claude".into()),
            created_at: None,
            last_activity_at: None,
        },
        SessionSummary {
            id: "sess-005".into(),
            project_id: "proj-alpha".into(),
            status: "merged".into(),
            activity: Some("exited".into()),
            branch: Some("fix/typo".into()),
            pr_url: Some("https://github.com/example/repo/pull/40".into()),
            pr_number: Some(40),
            agent_name: Some("claude".into()),
            created_at: None,
            last_activity_at: None,
        },
    ]
}

fn sample_pr_statuses() -> Vec<PrStatusEntry> {
    vec![
        PrStatusEntry {
            session_id: "sess-002".into(),
            pr_number: Some(42),
            pr_url: Some("https://github.com/example/repo/pull/42".into()),
            branch: Some("fix/db-timeout".into()),
            status: "ci_failed".into(),
            ci_status: Some("failing".into()),
            review_status: Some("pending".into()),
        },
        PrStatusEntry {
            session_id: "sess-003".into(),
            pr_number: Some(43),
            pr_url: Some("https://github.com/example/repo/pull/43".into()),
            branch: Some("feat/new-api".into()),
            status: "changes_requested".into(),
            ci_status: Some("passing".into()),
            review_status: Some("changes_requested".into()),
        },
        PrStatusEntry {
            session_id: "sess-004".into(),
            pr_number: Some(44),
            pr_url: Some("https://github.com/example/repo/pull/44".into()),
            branch: Some("feat/dashboard".into()),
            status: "approved".into(),
            ci_status: Some("passing".into()),
            review_status: Some("approved".into()),
        },
        PrStatusEntry {
            session_id: "sess-005".into(),
            pr_number: Some(40),
            pr_url: Some("https://github.com/example/repo/pull/40".into()),
            branch: Some("fix/typo".into()),
            status: "merged".into(),
            ci_status: Some("passing".into()),
            review_status: Some("approved".into()),
        },
    ]
}

const APP_STYLE: &str = "\
    min-height: 100vh; \
    background: #0f172a; \
    color: #f8fafc; \
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;";
