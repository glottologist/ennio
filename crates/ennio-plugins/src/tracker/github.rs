use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::id::ProjectId;
use ennio_core::tracker::{
    CreateIssueInput, Issue, IssueFilters, IssueState, IssueUpdate, Tracker,
};
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use tracing::debug;
use url::Url;

pub struct GitHubTracker {
    client: Client,
    token: String,
}

impl GitHubTracker {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
        }
    }
}

fn api_url(repo: &str, path: &str) -> Result<Url, EnnioError> {
    let base = format!("https://api.github.com/repos/{repo}{path}");
    Url::parse(&base).map_err(|e| EnnioError::Tracker {
        message: format!("invalid GitHub API URL: {e}"),
    })
}

fn parse_repo(project_id: &ProjectId) -> &str {
    project_id.as_str()
}

impl GitHubTracker {
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }
}

#[async_trait]
impl Tracker for GitHubTracker {
    fn name(&self) -> &str {
        "github"
    }

    async fn get_issue(&self, project_id: &ProjectId, issue_id: &str) -> Result<Issue, EnnioError> {
        let repo = parse_repo(project_id);
        let url = api_url(repo, &format!("/issues/{issue_id}"))?;

        debug!(repo = %repo, issue = %issue_id, "fetching GitHub issue");

        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.auth_header())
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "ennio")
            .send()
            .await
            .map_err(|e| EnnioError::Tracker {
                message: format!("request failed: {e}"),
            })?;

        if !response.status().is_success() {
            return Err(EnnioError::Tracker {
                message: format!("GitHub API returned {}", response.status()),
            });
        }

        let body: serde_json::Value = response.json().await.map_err(|e| EnnioError::Tracker {
            message: format!("failed to parse response: {e}"),
        })?;

        parse_issue(&body)
    }

    async fn is_completed(&self, issue: &Issue) -> Result<bool, EnnioError> {
        Ok(matches!(issue.state, IssueState::Done | IssueState::Closed))
    }

    fn issue_url(&self, project_id: &ProjectId, issue_id: &str) -> String {
        let repo = parse_repo(project_id);
        format!("https://github.com/{repo}/issues/{issue_id}")
    }

    fn branch_name(&self, issue: &Issue) -> String {
        let sanitized: String = issue
            .title
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();

        let trimmed = sanitized.trim_matches('-');
        let end = truncate_to_char_boundary(trimmed, 50);
        let truncated = &trimmed[..end];
        let truncated = truncated.trim_end_matches('-');

        if truncated.is_empty() {
            format!("issue-{}", issue.id)
        } else {
            format!("issue-{}-{truncated}", issue.id)
        }
    }

    fn generate_prompt(&self, issue: &Issue) -> String {
        let mut prompt = format!("Issue #{}: {}", issue.id, issue.title);

        if let Some(body) = &issue.body {
            if !body.is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(body);
            }
        }

        if !issue.labels.is_empty() {
            prompt.push_str("\n\nLabels: ");
            prompt.push_str(&issue.labels.join(", "));
        }

        prompt
    }

    async fn list_issues(
        &self,
        project_id: &ProjectId,
        filters: &IssueFilters,
    ) -> Result<Vec<Issue>, EnnioError> {
        let repo = parse_repo(project_id);
        let mut url = api_url(repo, "/issues")?;

        {
            let mut pairs = url.query_pairs_mut();
            if let Some(state) = &filters.state {
                let state_str = match state {
                    IssueState::Open | IssueState::InProgress => "open",
                    IssueState::Done | IssueState::Closed | IssueState::Cancelled => "closed",
                };
                pairs.append_pair("state", state_str);
            }
            if let Some(assignee) = &filters.assignee {
                pairs.append_pair("assignee", assignee);
            }
            if !filters.labels.is_empty() {
                pairs.append_pair("labels", &filters.labels.join(","));
            }
            if let Some(limit) = filters.limit {
                pairs.append_pair("per_page", &limit.to_string());
            }
        }

        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.auth_header())
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "ennio")
            .send()
            .await
            .map_err(|e| EnnioError::Tracker {
                message: format!("request failed: {e}"),
            })?;

        if !response.status().is_success() {
            return Err(EnnioError::Tracker {
                message: format!("GitHub API returned {}", response.status()),
            });
        }

        let body: Vec<serde_json::Value> =
            response.json().await.map_err(|e| EnnioError::Tracker {
                message: format!("failed to parse response: {e}"),
            })?;

        body.iter().map(parse_issue).collect()
    }

    async fn update_issue(
        &self,
        project_id: &ProjectId,
        issue_id: &str,
        update: &IssueUpdate,
    ) -> Result<(), EnnioError> {
        let repo = parse_repo(project_id);

        let mut patch = serde_json::Map::new();

        if let Some(state) = &update.state {
            let state_str = match state {
                IssueState::Open | IssueState::InProgress => "open",
                IssueState::Done | IssueState::Closed | IssueState::Cancelled => "closed",
            };
            patch.insert(
                "state".to_string(),
                serde_json::Value::String(state_str.to_string()),
            );
        }

        if let Some(labels) = &update.labels {
            let label_values: Vec<serde_json::Value> = labels
                .iter()
                .map(|l| serde_json::Value::String(l.to_string()))
                .collect();
            patch.insert("labels".to_string(), serde_json::Value::Array(label_values));
        }

        if let Some(assignees) = &update.assignees {
            let assignee_values: Vec<serde_json::Value> = assignees
                .iter()
                .map(|a| serde_json::Value::String(a.to_string()))
                .collect();
            patch.insert(
                "assignees".to_string(),
                serde_json::Value::Array(assignee_values),
            );
        }

        let url = api_url(repo, &format!("/issues/{issue_id}"))?;

        let response = self
            .client
            .patch(url)
            .header(AUTHORIZATION, self.auth_header())
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "ennio")
            .json(&serde_json::Value::Object(patch))
            .send()
            .await
            .map_err(|e| EnnioError::Tracker {
                message: format!("request failed: {e}"),
            })?;

        if !response.status().is_success() {
            return Err(EnnioError::Tracker {
                message: format!("GitHub API returned {}", response.status()),
            });
        }

        if let Some(comment) = &update.comment {
            let comment_url = api_url(repo, &format!("/issues/{issue_id}/comments"))?;
            let mut body = serde_json::Map::new();
            body.insert(
                "body".to_string(),
                serde_json::Value::String(comment.to_string()),
            );

            let comment_response = self
                .client
                .post(comment_url)
                .header(AUTHORIZATION, self.auth_header())
                .header(ACCEPT, "application/vnd.github+json")
                .header(USER_AGENT, "ennio")
                .json(&serde_json::Value::Object(body))
                .send()
                .await
                .map_err(|e| EnnioError::Tracker {
                    message: format!("comment request failed: {e}"),
                })?;

            if !comment_response.status().is_success() {
                return Err(EnnioError::Tracker {
                    message: format!("GitHub comment API returned {}", comment_response.status()),
                });
            }
        }

        Ok(())
    }

    async fn create_issue(
        &self,
        project_id: &ProjectId,
        input: &CreateIssueInput,
    ) -> Result<Issue, EnnioError> {
        let repo = parse_repo(project_id);
        let url = api_url(repo, "/issues")?;

        let mut body = serde_json::Map::new();
        body.insert(
            "title".to_string(),
            serde_json::Value::String(input.title.to_string()),
        );

        if let Some(desc) = &input.body {
            body.insert(
                "body".to_string(),
                serde_json::Value::String(desc.to_string()),
            );
        }

        if !input.labels.is_empty() {
            let labels: Vec<serde_json::Value> = input
                .labels
                .iter()
                .map(|l| serde_json::Value::String(l.to_string()))
                .collect();
            body.insert("labels".to_string(), serde_json::Value::Array(labels));
        }

        if !input.assignees.is_empty() {
            let assignees: Vec<serde_json::Value> = input
                .assignees
                .iter()
                .map(|a| serde_json::Value::String(a.to_string()))
                .collect();
            body.insert("assignees".to_string(), serde_json::Value::Array(assignees));
        }

        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, self.auth_header())
            .header(ACCEPT, "application/vnd.github+json")
            .header(USER_AGENT, "ennio")
            .json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(|e| EnnioError::Tracker {
                message: format!("request failed: {e}"),
            })?;

        if !response.status().is_success() {
            return Err(EnnioError::Tracker {
                message: format!("GitHub API returned {}", response.status()),
            });
        }

        let resp_body: serde_json::Value =
            response.json().await.map_err(|e| EnnioError::Tracker {
                message: format!("failed to parse response: {e}"),
            })?;

        parse_issue(&resp_body)
    }
}

fn parse_issue(value: &serde_json::Value) -> Result<Issue, EnnioError> {
    let id = value
        .get("number")
        .and_then(|v| v.as_u64())
        .map(|n| n.to_string())
        .ok_or_else(|| EnnioError::Tracker {
            message: "missing issue number".to_string(),
        })?;

    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let body = value.get("body").and_then(|v| v.as_str()).map(String::from);

    let state_str = value
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("open");

    let state = match state_str {
        "closed" => IssueState::Closed,
        _ => IssueState::Open,
    };

    let labels = value
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|l| l.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let assignees = value
        .get("assignees")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("login").and_then(|l| l.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let url = value
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Issue {
        id,
        title,
        body,
        state,
        labels,
        assignees,
        url,
    })
}

fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_github_issue_json() -> impl Strategy<Value = serde_json::Value> {
        (
            1u64..1_000_000,
            ".*",
            prop::option::of(".*"),
            prop::bool::ANY,
            prop::collection::vec(".*", 0..5),
            prop::collection::vec("[a-zA-Z0-9_-]+", 0..3),
        )
            .prop_map(|(number, title, body, closed, labels, assignees)| {
                let state = if closed { "closed" } else { "open" };
                let label_arr: Vec<serde_json::Value> = labels
                    .into_iter()
                    .map(|l| serde_json::json!({"name": l}))
                    .collect();
                let assignee_arr: Vec<serde_json::Value> = assignees
                    .into_iter()
                    .map(|a| serde_json::json!({"login": a}))
                    .collect();
                serde_json::json!({
                    "number": number,
                    "title": title,
                    "body": body,
                    "state": state,
                    "labels": label_arr,
                    "assignees": assignee_arr,
                    "html_url": format!("https://github.com/test/repo/issues/{number}")
                })
            })
    }

    proptest! {
        #[test]
        fn parse_issue_never_panics(json in arb_github_issue_json()) {
            let result = parse_issue(&json);
            prop_assert!(result.is_ok());
            let issue = result.unwrap();
            prop_assert!(!issue.id.is_empty());
        }

        #[test]
        fn parse_issue_state_roundtrip(closed in prop::bool::ANY) {
            let json = serde_json::json!({
                "number": 1,
                "title": "test",
                "state": if closed { "closed" } else { "open" },
                "labels": [],
                "assignees": []
            });
            let issue = parse_issue(&json).unwrap();
            if closed {
                prop_assert_eq!(issue.state, IssueState::Closed);
            } else {
                prop_assert_eq!(issue.state, IssueState::Open);
            }
        }

        #[test]
        fn parse_issue_missing_number_returns_error(title in ".*") {
            let json = serde_json::json!({
                "title": title,
                "state": "open",
                "labels": [],
                "assignees": []
            });
            prop_assert!(parse_issue(&json).is_err());
        }

        #[test]
        fn truncate_never_splits_utf8(s in "\\PC*", max in 0usize..200) {
            let boundary = truncate_to_char_boundary(&s, max);
            prop_assert!(s.is_char_boundary(boundary));
            prop_assert!(boundary <= max || boundary == 0);
            prop_assert!(boundary <= s.len());
        }

        #[test]
        fn branch_name_is_ascii_and_bounded(title in "[\\w\\s-]{1,100}", id in "[0-9]{1,6}") {
            let issue = Issue {
                id,
                title,
                body: None,
                state: IssueState::Open,
                labels: vec![],
                assignees: vec![],
                url: None,
            };
            let tracker = GitHubTracker::new("fake-token");
            let branch = tracker.branch_name(&issue);
            prop_assert!(branch.starts_with("issue-"));
            prop_assert!(branch.is_ascii());
            prop_assert!(!branch.ends_with('-'));
        }
    }
}
