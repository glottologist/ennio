use dioxus::prelude::*;

use crate::types::PrStatusEntry;

#[derive(Props, Clone, PartialEq)]
pub struct PrTableProps {
    pub entries: Vec<PrStatusEntry>,
}

#[component]
pub fn PrTable(props: PrTableProps) -> Element {
    let with_prs: Vec<&PrStatusEntry> = props
        .entries
        .iter()
        .filter(|e| e.pr_number.is_some())
        .collect();

    rsx! {
        div {
            style: SECTION_STYLE,
            h2 { style: SECTION_TITLE_STYLE, "PR Status" }
            if with_prs.is_empty() {
                p { style: EMPTY_STYLE, "No active pull requests." }
            } else {
                table {
                    style: TABLE_STYLE,
                    thead {
                        tr {
                            style: HEADER_ROW_STYLE,
                            th { style: TH_STYLE, "Session" }
                            th { style: TH_STYLE, "PR #" }
                            th { style: TH_STYLE, "Branch" }
                            th { style: TH_STYLE, "CI" }
                            th { style: TH_STYLE, "Review" }
                            th { style: TH_STYLE, "Link" }
                        }
                    }
                    tbody {
                        for entry in with_prs {
                            PrRow { entry: entry.clone() } // clone: owned value needed for child component props
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct PrRowProps {
    entry: PrStatusEntry,
}

#[component]
fn PrRow(props: PrRowProps) -> Element {
    let e = &props.entry;
    let ci_color = e.ci_color();

    rsx! {
        tr {
            style: ROW_STYLE,
            td { style: TD_STYLE, "{e.session_id}" }
            td { style: TD_STYLE,
                if let Some(num) = e.pr_number {
                    "#{num}"
                }
            }
            td { style: TD_STYLE,
                if let Some(ref branch) = e.branch {
                    "{branch}"
                } else {
                    "-"
                }
            }
            td { style: TD_STYLE,
                span {
                    style: "color: {ci_color}; font-weight: 500;",
                    "{e.ci_label()}"
                }
            }
            td { style: TD_STYLE, "{e.review_label()}" }
            td { style: TD_STYLE,
                if let Some(ref url) = e.pr_url {
                    a {
                        href: "{url}",
                        target: "_blank",
                        style: "color: #60a5fa; text-decoration: none;",
                        "View"
                    }
                } else {
                    span { "-" }
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

const TABLE_STYLE: &str = "\
    width: 100%; \
    border-collapse: collapse; \
    background: #1e293b; \
    border-radius: 0.5rem; \
    overflow: hidden;";

const HEADER_ROW_STYLE: &str = "\
    background: #334155;";

const TH_STYLE: &str = "\
    padding: 0.75rem 1rem; \
    text-align: left; \
    font-size: 0.75rem; \
    font-weight: 600; \
    color: #94a3b8; \
    text-transform: uppercase; \
    letter-spacing: 0.05em;";

const ROW_STYLE: &str = "\
    border-bottom: 1px solid #334155;";

const TD_STYLE: &str = "\
    padding: 0.75rem 1rem; \
    font-size: 0.8125rem; \
    color: #cbd5e1;";

const EMPTY_STYLE: &str = "\
    color: #64748b; \
    font-style: italic;";
