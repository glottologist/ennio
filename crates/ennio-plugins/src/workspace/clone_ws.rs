use std::path::{Path, PathBuf};

use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::id::ProjectId;
use ennio_core::workspace::{Workspace, WorkspaceCreateConfig, WorkspaceInfo};
use tokio::process::Command;
use tracing::debug;

use super::run_post_create_hooks;

pub struct CloneWorkspace;

impl CloneWorkspace {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CloneWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

fn clone_path(config: &WorkspaceCreateConfig<'_>) -> PathBuf {
    let parent = config.project.path.parent().unwrap_or(&config.project.path);
    parent.join(format!(
        ".ennio-clones/{}/{}",
        config.project.name, config.session_id
    ))
}

#[async_trait]
impl Workspace for CloneWorkspace {
    fn name(&self) -> &str {
        "clone"
    }

    async fn create(&self, config: &WorkspaceCreateConfig<'_>) -> Result<PathBuf, EnnioError> {
        let path = clone_path(config);

        debug!(
            repo = %config.project.repo,
            path = %path.display(),
            "cloning repository"
        );

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| EnnioError::Io {
                    path: Some(parent.to_path_buf()),
                    source: e,
                })?;
        }

        let path_str = path.to_string_lossy();
        let output = Command::new("git")
            .args(["clone", &config.project.repo, &path_str])
            .output()
            .await
            .map_err(|e| EnnioError::Workspace {
                message: format!("failed to clone: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnnioError::Workspace {
                message: format!("git clone failed: {stderr}"),
            });
        }

        if let Some(branch) = config.branch {
            let checkout_output = Command::new("git")
                .args(["checkout", "-b", branch])
                .current_dir(&path)
                .output()
                .await
                .map_err(|e| EnnioError::Workspace {
                    message: format!("failed to create branch: {e}"),
                })?;

            if !checkout_output.status.success() {
                let stderr = String::from_utf8_lossy(&checkout_output.stderr);
                return Err(EnnioError::Workspace {
                    message: format!("git checkout -b failed: {stderr}"),
                });
            }
        }

        Ok(path)
    }

    async fn destroy(&self, path: &Path) -> Result<(), EnnioError> {
        debug!(path = %path.display(), "removing cloned workspace");

        tokio::fs::remove_dir_all(path)
            .await
            .map_err(|e| EnnioError::Io {
                path: Some(path.to_path_buf()),
                source: e,
            })?;

        Ok(())
    }

    async fn list(&self, _project_id: &ProjectId) -> Result<Vec<WorkspaceInfo>, EnnioError> {
        Ok(Vec::new())
    }

    async fn post_create(
        &self,
        path: &Path,
        config: &WorkspaceCreateConfig<'_>,
    ) -> Result<(), EnnioError> {
        run_post_create_hooks(path, config).await
    }

    async fn exists(&self, path: &Path) -> Result<bool, EnnioError> {
        Ok(path.exists())
    }

    async fn restore(
        &self,
        path: &Path,
        config: &WorkspaceCreateConfig<'_>,
    ) -> Result<(), EnnioError> {
        if !path.exists() {
            return Err(EnnioError::Workspace {
                message: format!("clone path does not exist: {}", path.display()),
            });
        }

        self.post_create(path, config).await
    }
}
