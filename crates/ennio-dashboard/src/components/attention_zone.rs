use dioxus::prelude::*;

use crate::types::SessionSummary;

#[derive(Props, Clone, PartialEq)]
pub struct AttentionZoneProps {
    pub sessions: Vec<SessionSummary>,
}

#[component]
pub fn AttentionZone(props: AttentionZoneProps) -> Element {
    let attention_sessions: Vec<&SessionSummary> = props
        .sessions
        .iter()
        .filter(|s| s.needs_attention())
        .collect();

    rsx! {
        div {
            style: SECTION_STYLE,
            h2 { style: SECTION_TITLE_STYLE, "Attention Required" }
            if attention_sessions.is_empty() {
                p { style: ALL_CLEAR_STYLE, "All sessions are healthy." }
            } else {
                div {
                    style: ALERT_LIST_STYLE,
                    for session in attention_sessions {
                        AttentionItem { session: session.clone() } // clone: owned value needed for child component props
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AttentionItemProps {
    session: SessionSummary,
}

#[component]
fn AttentionItem(props: AttentionItemProps) -> Element {
    let s = &props.session;
    let reason = s.attention_reason().unwrap_or("Needs attention");
    let color = s.status_color();

    rsx! {
        div {
            style: "display: flex; align-items: center; gap: 1rem; padding: 0.75rem 1rem; \
                    background: {color}11; border-left: 3px solid {color}; border-radius: 0.25rem;",
            div {
                style: "flex: 1;",
                div {
                    style: "display: flex; justify-content: space-between; align-items: center;",
                    span { style: "font-weight: 600; color: #e2e8f0; font-size: 0.875rem;", "{s.id}" }
                    span {
                        style: "font-size: 0.75rem; padding: 0.125rem 0.5rem; border-radius: 9999px; \
                                background: {color}22; color: {color};",
                        "{s.status_label()}"
                    }
                }
                div {
                    style: "display: flex; gap: 1rem; margin-top: 0.25rem;",
                    span { style: DETAIL_STYLE, "Project: {s.project_id}" }
                    if let Some(ref branch) = s.branch {
                        span { style: DETAIL_STYLE, "Branch: {branch}" }
                    }
                }
                p {
                    style: "margin: 0.25rem 0 0 0; font-size: 0.8125rem; color: {color};",
                    "{reason}"
                }
            }
        }
    }
}

const SECTION_STYLE: &str = "\
    padding: 1.5rem 2rem;";

const SECTION_TITLE_STYLE: &str = "\
    font-size: 1.25rem; \
    font-weight: 600; \
    color: #fca5a5; \
    margin: 0 0 1rem 0;";

const ALERT_LIST_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    gap: 0.75rem;";

const ALL_CLEAR_STYLE: &str = "\
    color: #22c55e; \
    font-style: italic;";

const DETAIL_STYLE: &str = "\
    font-size: 0.75rem; \
    color: #94a3b8;";
