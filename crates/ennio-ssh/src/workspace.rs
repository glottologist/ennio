use std::path::{Path, PathBuf};

use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::id::ProjectId;
use ennio_core::workspace::{Workspace, WorkspaceCreateConfig, WorkspaceInfo};
use tracing::{debug, warn};

use crate::SshClient;
use crate::shell::escape;

pub struct SshWorktreeWorkspace {
    client: SshClient,
    repo_path: String,
}

impl SshWorktreeWorkspace {
    pub fn new(client: SshClient, repo_path: String) -> Self {
        Self { client, repo_path }
    }
}

fn worktree_path(config: &WorkspaceCreateConfig<'_>) -> String {
    let parent = config.project.path.parent().unwrap_or(&config.project.path);
    format!(
        "{}/.ennio-worktrees/{}/{}",
        parent.display(),
        config.project.name,
        config.session_id
    )
}

fn check_ssh_exit(output: &crate::client::ExecOutput, context: &str) -> Result<(), EnnioError> {
    match output.exit_code {
        Some(0) => Ok(()),
        Some(code) => Err(EnnioError::Workspace {
            message: format!("{context}: exit code {code}, stderr: {}", output.stderr),
        }),
        None => Err(EnnioError::Workspace {
            message: format!(
                "{context}: no exit code received, stderr: {}",
                output.stderr
            ),
        }),
    }
}

#[async_trait]
impl Workspace for SshWorktreeWorkspace {
    fn name(&self) -> &str {
        "ssh-worktree"
    }

    async fn create(&self, config: &WorkspaceCreateConfig<'_>) -> Result<PathBuf, EnnioError> {
        let path = worktree_path(config);
        let branch = config.branch.unwrap_or_else(|| config.session_id.as_str());

        debug!(
            path = %path,
            branch = %branch,
            "creating remote git worktree via SSH"
        );

        let escaped_repo = escape(&self.repo_path);
        let escaped_path = escape(&path);
        let escaped_branch = escape(branch);

        let cmd = format!("git -C {escaped_repo} worktree add {escaped_path} -b {escaped_branch}");

        let output = self.client.exec(&cmd).await.map_err(EnnioError::from)?;

        check_ssh_exit(&output, "git worktree add failed")?;

        Ok(PathBuf::from(path))
    }

    async fn destroy(&self, path: &Path) -> Result<(), EnnioError> {
        debug!(path = %path.display(), "destroying remote git worktree via SSH");

        let escaped_repo = escape(&self.repo_path);
        let path_str = path.to_string_lossy();
        let escaped_path = escape(&path_str);

        let cmd = format!("git -C {escaped_repo} worktree remove {escaped_path} --force");

        let output = self.client.exec(&cmd).await.map_err(EnnioError::from)?;

        check_ssh_exit(&output, "git worktree remove failed")?;

        Ok(())
    }

    async fn list(&self, _project_id: &ProjectId) -> Result<Vec<WorkspaceInfo>, EnnioError> {
        let escaped_repo = escape(&self.repo_path);
        let cmd = format!("git -C {escaped_repo} worktree list --porcelain");

        let output = self.client.exec(&cmd).await.map_err(EnnioError::from)?;

        if output.exit_code != Some(0) {
            return Ok(Vec::new());
        }

        Ok(parse_porcelain_output(&output.stdout))
    }

    async fn post_create(
        &self,
        path: &Path,
        config: &WorkspaceCreateConfig<'_>,
    ) -> Result<(), EnnioError> {
        if !config.project.symlinks.is_empty() {
            warn!(
                "skipping {} symlink(s) for remote workspace — symlinks are local-path concepts",
                config.project.symlinks.len()
            );
        }

        let path_str = path.to_string_lossy();
        let escaped_path = escape(&path_str);

        for cmd_str in &config.project.post_create {
            debug!(command = %cmd_str, "running remote post-create command via SSH");

            let escaped_cmd = escape(cmd_str);
            let remote_cmd = format!("cd {escaped_path} && sh -c {escaped_cmd}");

            let output = self
                .client
                .exec(&remote_cmd)
                .await
                .map_err(EnnioError::from)?;

            check_ssh_exit(&output, &format!("post-create command '{cmd_str}' failed"))?;
        }

        Ok(())
    }

    async fn exists(&self, path: &Path) -> Result<bool, EnnioError> {
        let path_str = path.to_string_lossy();
        let escaped_path = escape(&path_str);
        let cmd = format!("test -d {escaped_path}");

        let output = self.client.exec(&cmd).await.map_err(EnnioError::from)?;

        Ok(output.exit_code == Some(0))
    }

    async fn restore(
        &self,
        path: &Path,
        config: &WorkspaceCreateConfig<'_>,
    ) -> Result<(), EnnioError> {
        if !self.exists(path).await? {
            let display = path.display();
            return Err(EnnioError::Workspace {
                message: format!("remote worktree path does not exist: {display}"),
            });
        }

        self.post_create(path, config).await
    }
}

fn parse_porcelain_output(output: &str) -> Vec<WorkspaceInfo> {
    let fallback_session_id = ennio_core::id::SessionId::new("unknown")
        .expect("hardcoded 'unknown' is valid per SessionId rules");

    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path_str) = line.strip_prefix("worktree ") {
            if let Some(path) = current_path.take() {
                worktrees.push(WorkspaceInfo {
                    path,
                    branch: current_branch.take(),
                    // clone: SessionId must be owned per WorkspaceInfo
                    session_id: fallback_session_id.clone(),
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
            // clone: SessionId must be owned per WorkspaceInfo
            session_id: fallback_session_id.clone(),
        });
    }

    worktrees
}

pub struct SshCloneWorkspace {
    client: SshClient,
}

impl SshCloneWorkspace {
    pub fn new(client: SshClient) -> Self {
        Self { client }
    }
}

fn clone_path(config: &WorkspaceCreateConfig<'_>) -> String {
    let parent = config.project.path.parent().unwrap_or(&config.project.path);
    format!(
        "{}/.ennio-clones/{}/{}",
        parent.display(),
        config.project.name,
        config.session_id
    )
}

#[async_trait]
impl Workspace for SshCloneWorkspace {
    fn name(&self) -> &str {
        "ssh-clone"
    }

    async fn create(&self, config: &WorkspaceCreateConfig<'_>) -> Result<PathBuf, EnnioError> {
        let path = clone_path(config);

        debug!(
            repo = %config.project.repo,
            path = %path,
            "cloning repository on remote via SSH"
        );

        let escaped_repo = escape(&config.project.repo);
        let escaped_path = escape(&path);

        let cmd = format!("git clone {escaped_repo} {escaped_path}");
        let output = self.client.exec(&cmd).await.map_err(EnnioError::from)?;

        check_ssh_exit(&output, "git clone failed")?;

        if let Some(branch) = config.branch {
            let escaped_branch = escape(branch);
            let checkout_cmd = format!("git -C {escaped_path} checkout -b {escaped_branch}");
            let checkout_output = self
                .client
                .exec(&checkout_cmd)
                .await
                .map_err(EnnioError::from)?;

            check_ssh_exit(&checkout_output, "git checkout -b failed")?;
        }

        Ok(PathBuf::from(path))
    }

    async fn destroy(&self, path: &Path) -> Result<(), EnnioError> {
        debug!(path = %path.display(), "removing remote cloned workspace via SSH");

        let path_str = path.to_string_lossy();
        let escaped_path = escape(&path_str);
        let cmd = format!("rm -rf {escaped_path}");

        let output = self.client.exec(&cmd).await.map_err(EnnioError::from)?;

        check_ssh_exit(&output, "rm -rf failed")?;

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
        if !config.project.symlinks.is_empty() {
            warn!(
                "skipping {} symlink(s) for remote workspace — symlinks are local-path concepts",
                config.project.symlinks.len()
            );
        }

        let path_str = path.to_string_lossy();
        let escaped_path = escape(&path_str);

        for cmd_str in &config.project.post_create {
            debug!(command = %cmd_str, "running remote post-create command via SSH");

            let escaped_cmd = escape(cmd_str);
            let remote_cmd = format!("cd {escaped_path} && sh -c {escaped_cmd}");

            let output = self
                .client
                .exec(&remote_cmd)
                .await
                .map_err(EnnioError::from)?;

            check_ssh_exit(&output, &format!("post-create command '{cmd_str}' failed"))?;
        }

        Ok(())
    }

    async fn exists(&self, path: &Path) -> Result<bool, EnnioError> {
        let path_str = path.to_string_lossy();
        let escaped_path = escape(&path_str);
        let cmd = format!("test -d {escaped_path}/.git");

        let output = self.client.exec(&cmd).await.map_err(EnnioError::from)?;

        Ok(output.exit_code == Some(0))
    }

    async fn restore(
        &self,
        path: &Path,
        config: &WorkspaceCreateConfig<'_>,
    ) -> Result<(), EnnioError> {
        if !self.exists(path).await? {
            let display = path.display();
            return Err(EnnioError::Workspace {
                message: format!("remote clone path does not exist: {display}"),
            });
        }

        self.post_create(path, config).await
    }
}

#[cfg(test)]
mod tests {
    use ennio_core::config::ProjectConfig;
    use ennio_core::id::{ProjectId, SessionId};
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    fn make_config<'a>(
        project: &'a ProjectConfig,
        project_id: &'a ProjectId,
        session_id: &'a SessionId,
        branch: Option<&'a str>,
    ) -> WorkspaceCreateConfig<'a> {
        WorkspaceCreateConfig {
            project_id,
            project,
            session_id,
            branch,
        }
    }

    fn make_exec_output(exit_code: Option<u32>, stderr: &str) -> crate::client::ExecOutput {
        crate::client::ExecOutput {
            stdout: String::new(),
            stderr: stderr.to_owned(),
            exit_code,
        }
    }

    #[test]
    fn check_ssh_exit_success() {
        let output = make_exec_output(Some(0), "");
        assert!(check_ssh_exit(&output, "test").is_ok());
    }

    #[rstest]
    #[case(Some(1), "non-zero exit")]
    #[case(Some(127), "command not found")]
    #[case(Some(255), "ssh error")]
    fn check_ssh_exit_nonzero(#[case] code: Option<u32>, #[case] context: &str) {
        let output = make_exec_output(code, "some error");
        let err = check_ssh_exit(&output, context).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(context), "error should contain context: {msg}");
        assert!(
            msg.contains("some error"),
            "error should contain stderr: {msg}"
        );
    }

    #[test]
    fn check_ssh_exit_no_exit_code() {
        let output = make_exec_output(None, "connection lost");
        let err = check_ssh_exit(&output, "remote cmd").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no exit code"),
            "should mention no exit code: {msg}"
        );
    }

    #[test]
    fn worktree_path_format() {
        let project = ProjectConfig {
            name: "myproj".to_owned(),
            path: PathBuf::from("/home/user/repos/myproj"),
            ..Default::default()
        };
        let pid = ProjectId::new("myproj").unwrap();
        let sid = SessionId::new("sess-001").unwrap();
        let config = make_config(&project, &pid, &sid, None);

        let path = worktree_path(&config);
        assert_eq!(path, "/home/user/repos/.ennio-worktrees/myproj/sess-001");
    }

    #[test]
    fn clone_path_format() {
        let project = ProjectConfig {
            name: "myproj".to_owned(),
            path: PathBuf::from("/home/user/repos/myproj"),
            ..Default::default()
        };
        let pid = ProjectId::new("myproj").unwrap();
        let sid = SessionId::new("sess-002").unwrap();
        let config = make_config(&project, &pid, &sid, None);

        let path = clone_path(&config);
        assert_eq!(path, "/home/user/repos/.ennio-clones/myproj/sess-002");
    }

    #[test]
    fn worktree_path_root_project() {
        let project = ProjectConfig {
            name: "rootproj".to_owned(),
            path: PathBuf::from("/rootproj"),
            ..Default::default()
        };
        let pid = ProjectId::new("rootproj").unwrap();
        let sid = SessionId::new("s1").unwrap();
        let config = make_config(&project, &pid, &sid, None);

        let path = worktree_path(&config);
        assert_eq!(path, "//.ennio-worktrees/rootproj/s1");
    }

    #[test]
    fn parse_porcelain_empty() {
        let result = parse_porcelain_output("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_porcelain_single_worktree() {
        let output = "worktree /home/user/repo\nbranch refs/heads/main\n";
        let result = parse_porcelain_output(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("/home/user/repo"));
        assert_eq!(result[0].branch.as_deref(), Some("main"));
    }

    #[test]
    fn parse_porcelain_multiple_worktrees() {
        let output = "\
worktree /home/user/repo
branch refs/heads/main

worktree /home/user/repo-wt-1
branch refs/heads/feature-a
";
        let result = parse_porcelain_output(output);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, PathBuf::from("/home/user/repo"));
        assert_eq!(result[0].branch.as_deref(), Some("main"));
        assert_eq!(result[1].path, PathBuf::from("/home/user/repo-wt-1"));
        assert_eq!(result[1].branch.as_deref(), Some("feature-a"));
    }

    #[test]
    fn parse_porcelain_worktree_without_branch() {
        let output = "worktree /tmp/detached\n";
        let result = parse_porcelain_output(output);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("/tmp/detached"));
        assert!(result[0].branch.is_none());
    }

    proptest! {
        #[test]
        fn check_ssh_exit_nonzero_always_errors(code in 1u32..=255) {
            let output = make_exec_output(Some(code), "err");
            prop_assert!(check_ssh_exit(&output, "ctx").is_err());
        }

        #[test]
        fn worktree_path_contains_session_id(
            name in "[a-z]{3,10}",
            session in "[a-z0-9-]{3,15}",
        ) {
            let project = ProjectConfig {
                name,
                path: PathBuf::from("/home/user/project"),
                ..Default::default()
            };
            let pid = ProjectId::new("proj").unwrap();
            let sid = SessionId::new(&session).unwrap();
            let config = make_config(&project, &pid, &sid, None);

            let path = worktree_path(&config);
            prop_assert!(path.contains(&session));
            prop_assert!(path.contains(".ennio-worktrees"));
        }

        #[test]
        fn clone_path_contains_session_id(
            name in "[a-z]{3,10}",
            session in "[a-z0-9-]{3,15}",
        ) {
            let project = ProjectConfig {
                name,
                path: PathBuf::from("/home/user/project"),
                ..Default::default()
            };
            let pid = ProjectId::new("proj").unwrap();
            let sid = SessionId::new(&session).unwrap();
            let config = make_config(&project, &pid, &sid, None);

            let path = clone_path(&config);
            prop_assert!(path.contains(&session));
            prop_assert!(path.contains(".ennio-clones"));
        }
    }
}
