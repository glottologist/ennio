use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::error::EnnioError;
use crate::id::ProjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PRState {
    Open,
    Draft,
    Closed,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRInfo {
    pub number: i32,
    pub title: String,
    pub url: String,
    pub state: PRState,
    pub branch: String,
    pub base_branch: String,
    pub author: String,
    pub draft: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CIStatus {
    Pending,
    Running,
    Passing,
    Failing,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CICheck {
    pub name: String,
    pub status: CIStatus,
    pub url: Option<String>,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    Pending,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub author: String,
    pub state: ReviewDecision,
    pub body: Option<String>,
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub author: String,
    pub body: String,
    pub path: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReadiness {
    pub mergeable: bool,
    pub ci_passing: bool,
    pub approved: bool,
    pub no_conflicts: bool,
    pub reasons: Vec<String>,
}

#[async_trait]
pub trait Scm: Send + Sync {
    fn name(&self) -> &str;

    async fn detect_pr(
        &self,
        project_id: &ProjectId,
        branch: &str,
    ) -> Result<Option<PRInfo>, EnnioError>;

    async fn get_pr_state(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<PRState, EnnioError>;

    async fn merge_pr(&self, project_id: &ProjectId, pr_number: i32) -> Result<(), EnnioError>;

    async fn close_pr(&self, project_id: &ProjectId, pr_number: i32) -> Result<(), EnnioError>;

    async fn get_ci_checks(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<Vec<CICheck>, EnnioError>;

    async fn get_ci_summary(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<CIStatus, EnnioError>;

    async fn get_reviews(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<Vec<Review>, EnnioError>;

    async fn get_review_decision(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<ReviewDecision, EnnioError>;

    async fn get_pending_comments(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<Vec<ReviewComment>, EnnioError>;

    async fn get_mergeability(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<MergeReadiness, EnnioError>;
}
