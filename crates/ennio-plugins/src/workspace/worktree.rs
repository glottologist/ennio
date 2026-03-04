use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::id::{ProjectId, SessionId};
use ennio_core::workspace::{Workspace, WorkspaceCreateConfig, WorkspaceInfo};
use tokio::process::Command;
use tracing::debug;

use super::run_post_create_hooks;

static FALLBACK_SESSION_ID: LazyLock<SessionId> = LazyLock::new(|| {
    SessionId::new("unknown").expect("hardcoded 'unknown' is valid per SessionId rules")
});

pub struct WorktreeWorkspace;

impl WorktreeWorkspace {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorktreeWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_git(cwd: &Path, args: &[&str]) -> Result<std::process::Output, EnnioError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| EnnioError::Workspace {
            message: format!("failed to execute git: {e}"),
        })?;
    Ok(output)
}

fn check_git_exit(output: &std::process::Output, context: &str) -> Result<(), EnnioError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(EnnioError::Workspace {
            message: format!("{context}: {stderr}"),
        });
    }
    Ok(())
}

fn worktree_path(config: &WorkspaceCreateConfig<'_>) -> PathBuf {
    let parent = config.project.path.parent().unwrap_or(&config.project.path);
    parent.join(format!(
        ".ennio-worktrees/{}/{}",
        config.project.name, config.session_id
    ))
}

#[async_trait]
impl Workspace for WorktreeWorkspace {
    fn name(&self) -> &str {
        "worktree"
    }

    async fn create(&self, config: &WorkspaceCreateConfig<'_>) -> Result<PathBuf, EnnioError> {
        let path = worktree_path(config);
        let branch = config.branch.unwrap_or_else(|| config.session_id.as_str());

        debug!(
            path = %path.display(),
            branch = %branch,
            "creating git worktree"
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
        let output = run_git(
            &config.project.path,
            &["worktree", "add", &path_str, "-b", branch],
        )
        .await?;

        check_git_exit(&output, "git worktree add failed")?;

        Ok(path)
    }

    async fn destroy(&self, path: &Path) -> Result<(), EnnioError> {
        debug!(path = %path.display(), "destroying git worktree");

        let repo_root = find_repo_root(path).await?;

        let path_str = path.to_string_lossy();
        let output = run_git(&repo_root, &["worktree", "remove", &path_str, "--force"]).await?;

        check_git_exit(&output, "git worktree remove failed")?;

        Ok(())
    }

    async fn list(&self, project_id: &ProjectId) -> Result<Vec<WorkspaceInfo>, EnnioError> {
        let base = dirs_home().join(format!(".ennio-worktrees/{project_id}"));

        if !base.exists() {
            return Ok(Vec::new());
        }

        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&base)
            .output()
            .await
            .map_err(|e| EnnioError::Workspace {
                message: format!("failed to list worktrees: {e}"),
            })?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let worktrees = parse_porcelain_output(&stdout, project_id);

        Ok(worktrees)
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
                message: format!("worktree path does not exist: {}", path.display()),
            });
        }

        self.post_create(path, config).await
    }
}

async fn find_repo_root(path: &Path) -> Result<PathBuf, EnnioError> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .await
        .map_err(|e| EnnioError::Workspace {
            message: format!("failed to find repo root: {e}"),
        })?;

    if !output.status.success() {
        return Err(EnnioError::Workspace {
            message: format!(
                "not a git repo: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

fn parse_porcelain_output(output: &str, _project_id: &ProjectId) -> Vec<WorkspaceInfo> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path_str) = line.strip_prefix("worktree ") {
            if let Some(path) = current_path.take() {
                worktrees.push(WorkspaceInfo {
                    path,
                    branch: current_branch.take(),
                    session_id: FALLBACK_SESSION_ID.clone(), // clone: SessionId must be owned per WorkspaceInfo
                });
            }
            current_path = Some(PathBuf::from(path_str));
        } else if let Some(branch_ref) = line.strip_prefix("branch refs/heads/") {
            current_branch = Some(branch_ref.to_string());
        }
    }

    if let Some(path) = current_path {
        worktrees.push(WorkspaceInfo {
            path,
            branch: current_branch,
            session_id: FALLBACK_SESSION_ID.clone(), // clone: SessionId must be owned per WorkspaceInfo
        });
    }

    worktrees
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}
