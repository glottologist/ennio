use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::error::EnnioError;
use crate::id::ProjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    Open,
    InProgress,
    Done,
    Closed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    pub state: IssueState,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IssueFilters {
    pub state: Option<IssueState>,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueUpdate {
    pub state: Option<IssueState>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIssueInput {
    pub title: String,
    pub body: Option<String>,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
}

#[async_trait]
pub trait Tracker: Send + Sync {
    fn name(&self) -> &str;

    async fn get_issue(&self, project_id: &ProjectId, issue_id: &str) -> Result<Issue, EnnioError>;

    async fn is_completed(&self, issue: &Issue) -> Result<bool, EnnioError>;

    fn issue_url(&self, project_id: &ProjectId, issue_id: &str) -> Result<String, EnnioError>;

    fn branch_name(&self, issue: &Issue) -> String;

    fn generate_prompt(&self, issue: &Issue) -> String;

    async fn list_issues(
        &self,
        project_id: &ProjectId,
        filters: &IssueFilters,
    ) -> Result<Vec<Issue>, EnnioError>;

    async fn update_issue(
        &self,
        project_id: &ProjectId,
        issue_id: &str,
        update: &IssueUpdate,
    ) -> Result<(), EnnioError>;

    async fn create_issue(
        &self,
        project_id: &ProjectId,
        input: &CreateIssueInput,
    ) -> Result<Issue, EnnioError>;
}
