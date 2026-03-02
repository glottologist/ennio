use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::id::ProjectId;
use ennio_core::tracker::{
    CreateIssueInput, Issue, IssueFilters, IssueState, IssueUpdate, Tracker,
};

pub struct LinearTracker;

impl LinearTracker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinearTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tracker for LinearTracker {
    fn name(&self) -> &str {
        "linear"
    }

    async fn get_issue(
        &self,
        _project_id: &ProjectId,
        _issue_id: &str,
    ) -> Result<Issue, EnnioError> {
        Err(EnnioError::Plugin {
            plugin: "linear".to_string(),
            message: "linear tracker not yet implemented".to_string(),
        })
    }

    async fn is_completed(&self, issue: &Issue) -> Result<bool, EnnioError> {
        Ok(matches!(
            issue.state,
            IssueState::Done | IssueState::Closed | IssueState::Cancelled
        ))
    }

    fn issue_url(&self, _project_id: &ProjectId, issue_id: &str) -> String {
        format!("https://linear.app/issue/{issue_id}")
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

        format!("{}-{truncated}", issue.id.to_lowercase())
    }

    fn generate_prompt(&self, issue: &Issue) -> String {
        let mut prompt = format!("{}: {}", issue.id, issue.title);

        if let Some(body) = &issue.body {
            if !body.is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(body);
            }
        }

        prompt
    }

    async fn list_issues(
        &self,
        _project_id: &ProjectId,
        _filters: &IssueFilters,
    ) -> Result<Vec<Issue>, EnnioError> {
        Err(EnnioError::Plugin {
            plugin: "linear".to_string(),
            message: "linear tracker not yet implemented".to_string(),
        })
    }

    async fn update_issue(
        &self,
        _project_id: &ProjectId,
        _issue_id: &str,
        _update: &IssueUpdate,
    ) -> Result<(), EnnioError> {
        Err(EnnioError::Plugin {
            plugin: "linear".to_string(),
            message: "linear tracker not yet implemented".to_string(),
        })
    }

    async fn create_issue(
        &self,
        _project_id: &ProjectId,
        _input: &CreateIssueInput,
    ) -> Result<Issue, EnnioError> {
        Err(EnnioError::Plugin {
            plugin: "linear".to_string(),
            message: "linear tracker not yet implemented".to_string(),
        })
    }
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
