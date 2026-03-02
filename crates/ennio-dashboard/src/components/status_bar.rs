use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct StatusBarProps {
    pub total: usize,
    pub active: usize,
    pub attention: usize,
    pub terminal: usize,
}

#[component]
pub fn StatusBar(props: StatusBarProps) -> Element {
    rsx! {
        div {
            style: STATUS_BAR_STYLE,
            h1 { style: TITLE_STYLE, "Ennio Dashboard" }
            div {
                style: COUNTERS_STYLE,
                StatusCounter { label: "Total", count: props.total, color: "#3b82f6" }
                StatusCounter { label: "Active", count: props.active, color: "#22c55e" }
                StatusCounter { label: "Attention", count: props.attention, color: "#ef4444" }
                StatusCounter { label: "Completed", count: props.terminal, color: "#6b7280" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct StatusCounterProps {
    label: &'static str,
    count: usize,
    color: &'static str,
}

#[component]
fn StatusCounter(props: StatusCounterProps) -> Element {
    rsx! {
        div {
            style: COUNTER_STYLE,
            span {
                style: "font-size: 2rem; font-weight: bold; color: {props.color};",
                "{props.count}"
            }
            span {
                style: COUNTER_LABEL_STYLE,
                "{props.label}"
            }
        }
    }
}

const STATUS_BAR_STYLE: &str = "\
    display: flex; \
    justify-content: space-between; \
    align-items: center; \
    padding: 1rem 2rem; \
    background: #1e293b; \
    border-bottom: 2px solid #334155; \
    color: #f8fafc;";

const TITLE_STYLE: &str = "\
    font-size: 1.5rem; \
    font-weight: 700; \
    margin: 0;";

const COUNTERS_STYLE: &str = "\
    display: flex; \
    gap: 2rem;";

const COUNTER_STYLE: &str = "\
    display: flex; \
    flex-direction: column; \
    align-items: center; \
    min-width: 5rem;";

const COUNTER_LABEL_STYLE: &str = "\
    font-size: 0.75rem; \
    color: #94a3b8; \
    text-transform: uppercase; \
    letter-spacing: 0.05em;";
