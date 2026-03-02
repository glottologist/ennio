use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::config::ProjectConfig;
use crate::error::EnnioError;
use crate::id::{ProjectId, SessionId};

#[derive(Debug, Clone)]
pub struct WorkspaceCreateConfig<'a> {
    pub project_id: &'a ProjectId,
    pub project: &'a ProjectConfig,
    pub session_id: &'a SessionId,
    pub branch: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub session_id: SessionId,
}

#[async_trait]
pub trait Workspace: Send + Sync {
    fn name(&self) -> &str;

    async fn create(&self, config: &WorkspaceCreateConfig<'_>) -> Result<PathBuf, EnnioError>;

    async fn destroy(&self, path: &Path) -> Result<(), EnnioError>;

    async fn list(&self, project_id: &ProjectId) -> Result<Vec<WorkspaceInfo>, EnnioError>;

    async fn post_create(
        &self,
        path: &Path,
        config: &WorkspaceCreateConfig<'_>,
    ) -> Result<(), EnnioError>;

    async fn exists(&self, path: &Path) -> Result<bool, EnnioError>;

    async fn restore(
        &self,
        path: &Path,
        config: &WorkspaceCreateConfig<'_>,
    ) -> Result<(), EnnioError>;
}
