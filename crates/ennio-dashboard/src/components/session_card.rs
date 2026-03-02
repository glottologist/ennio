use dioxus::prelude::*;

use crate::types::SessionSummary;

#[derive(Props, Clone, PartialEq)]
pub struct SessionCardProps {
    pub session: SessionSummary,
}

#[component]
pub fn SessionCard(props: SessionCardProps) -> Element {
    let s = &props.session;
    let border_color = s.status_color();

    rsx! {
        div {
            style: "background: #1e293b; border-radius: 0.5rem; padding: 1rem; \
                    border-left: 4px solid {border_color}; \
                    box-shadow: 0 1px 3px rgba(0,0,0,0.3);",
            CardHeader { id: s.id.as_str(), status_label: s.status_label(), status_color: border_color }
            CardBody { session: props.session }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct CardHeaderProps {
    id: String,
    status_label: &'static str,
    status_color: &'static str,
}

#[component]
fn CardHeader(props: CardHeaderProps) -> Element {
    rsx! {
        div {
            style: CARD_HEADER_STYLE,
            span {
                style: "font-weight: 600; font-size: 0.875rem; color: #e2e8f0;",
                "{props.id}"
            }
            span {
                style: "padding: 0.125rem 0.5rem; border-radius: 9999px; \
                        font-size: 0.75rem; font-weight: 500; \
                        background: {props.status_color}22; \
                        color: {props.status_color};",
                "{props.status_label}"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct CardBodyProps {
    session: SessionSummary,
}

#[component]
fn CardBody(props: CardBodyProps) -> Element {
    let s = &props.session;

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 0.375rem; margin-top: 0.75rem;",
            CardField { label: "Project", value: s.project_id.as_str() }
            CardField { label: "Activity", value: s.activity_label() }
            if let Some(ref branch) = s.branch {
                CardField { label: "Branch", value: branch.as_str() }
            }
            if let Some(ref agent) = s.agent_name {
                CardField { label: "Agent", value: agent.as_str() }
            }
            PrUrlField { pr_url: s.pr_url.clone() } // clone: moving optional String into child component props
            TimestampFields { created_at: s.created_at, last_activity_at: s.last_activity_at }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct CardFieldProps {
    label: &'static str,
    value: String,
}

#[component]
fn CardField(props: CardFieldProps) -> Element {
    rsx! {
        div {
            style: FIELD_ROW_STYLE,
            span { style: FIELD_LABEL_STYLE, "{props.label}" }
            span { style: FIELD_VALUE_STYLE, "{props.value}" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct PrUrlFieldProps {
    pr_url: Option<String>,
}

#[component]
fn PrUrlField(props: PrUrlFieldProps) -> Element {
    match props.pr_url {
        Some(ref url) => rsx! {
            div {
                style: FIELD_ROW_STYLE,
                span { style: FIELD_LABEL_STYLE, "PR" }
                a {
                    href: "{url}",
                    target: "_blank",
                    style: "color: #60a5fa; text-decoration: none; font-size: 0.8125rem;",
                    "{url}"
                }
            }
        },
        None => rsx! {},
    }
}

#[derive(Props, Clone, PartialEq)]
struct TimestampFieldsProps {
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    last_activity_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[component]
fn TimestampFields(props: TimestampFieldsProps) -> Element {
    rsx! {
        if let Some(created) = props.created_at {
            div {
                style: FIELD_ROW_STYLE,
                span { style: FIELD_LABEL_STYLE, "Created" }
                span { style: FIELD_VALUE_STYLE, "{created}" }
            }
        }
        if let Some(last) = props.last_activity_at {
            div {
                style: FIELD_ROW_STYLE,
                span { style: FIELD_LABEL_STYLE, "Last Active" }
                span { style: FIELD_VALUE_STYLE, "{last}" }
            }
        }
    }
}

const CARD_HEADER_STYLE: &str = "\
    display: flex; \
    justify-content: space-between; \
    align-items: center;";

const FIELD_ROW_STYLE: &str = "\
    display: flex; \
    justify-content: space-between; \
    align-items: center;";

const FIELD_LABEL_STYLE: &str = "\
    font-size: 0.75rem; \
    color: #94a3b8; \
    text-transform: uppercase;";

const FIELD_VALUE_STYLE: &str = "\
    font-size: 0.8125rem; \
    color: #cbd5e1;";
