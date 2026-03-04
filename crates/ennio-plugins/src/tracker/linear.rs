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

    fn issue_url(&self, _project_id: &ProjectId, issue_id: &str) -> Result<String, EnnioError> {
        let mut url = url::Url::parse("https://linear.app").map_err(|e| EnnioError::Tracker {
            message: format!("invalid base URL: {e}"),
        })?;
        url.path_segments_mut()
            .map_err(|()| EnnioError::Tracker {
                message: "cannot-be-a-base URL".to_string(),
            })?
            .push("issue")
            .push(issue_id);
        Ok(url.to_string())
    }

    fn branch_name(&self, issue: &Issue) -> String {
        let sanitized = sanitize_title_for_branch(&issue.title, 50);
        format!("{}-{sanitized}", issue.id.to_lowercase())
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

use super::sanitize_title_for_branch;
