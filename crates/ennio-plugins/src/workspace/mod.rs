mod clone_ws;
mod worktree;

use std::path::Path;

use ennio_core::error::EnnioError;
use ennio_core::workspace::WorkspaceCreateConfig;
use tokio::process::Command;
use tracing::debug;

pub use clone_ws::CloneWorkspace;
pub use worktree::WorktreeWorkspace;

pub(crate) async fn run_post_create_hooks(
    path: &Path,
    config: &WorkspaceCreateConfig<'_>,
) -> Result<(), EnnioError> {
    for symlink in &config.project.symlinks {
        let source = &symlink.source;
        let target = path.join(&symlink.target);

        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| EnnioError::Io {
                    path: Some(parent.to_path_buf()),
                    source: e,
                })?;
        }

        debug!(
            source = %source.display(),
            target = %target.display(),
            "creating symlink"
        );

        tokio::fs::symlink(source, &target)
            .await
            .map_err(|e| EnnioError::Io {
                path: Some(target),
                source: e,
            })?;
    }

    for cmd_str in &config.project.post_create {
        debug!(command = %cmd_str, "running post-create command");

        let output = Command::new("sh")
            .args(["-c", cmd_str])
            .current_dir(path)
            .output()
            .await
            .map_err(|e| EnnioError::Workspace {
                message: format!("post-create command failed: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnnioError::Workspace {
                message: format!("post-create command '{cmd_str}' failed: {stderr}"),
            });
        }
    }

    Ok(())
}
