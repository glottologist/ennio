use dioxus::prelude::*;

use crate::components::session_card::SessionCard;
use crate::types::SessionSummary;

#[derive(Props, Clone, PartialEq)]
pub struct SessionListProps {
    pub sessions: Vec<SessionSummary>,
}

#[component]
pub fn SessionList(props: SessionListProps) -> Element {
    rsx! {
        div {
            style: SECTION_STYLE,
            h2 { style: SECTION_TITLE_STYLE, "Sessions" }
            if props.sessions.is_empty() {
                p { style: EMPTY_STYLE, "No sessions found." }
            } else {
                div {
                    style: GRID_STYLE,
                    for session in props.sessions {
                        SessionCard {
                            key: "{session.id}",
                            session: session,
                        }
                    }
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
    color: #f1f5f9; \
    margin: 0 0 1rem 0;";

const GRID_STYLE: &str = "\
    display: grid; \
    grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr)); \
    gap: 1rem;";

const EMPTY_STYLE: &str = "\
    color: #64748b; \
    font-style: italic;";
