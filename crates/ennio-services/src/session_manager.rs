use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use ennio_core::agent::{AgentLaunchConfig, PromptDelivery, WorkspaceHooksConfig};
use ennio_core::config::{
    OrchestratorConfig, ProjectConfig, SshConnectionConfig, SshStrategyConfig,
};
use ennio_core::error::EnnioError;
use ennio_core::event::{EventPriority, EventType};
use ennio_core::id::{ProjectId, SessionId};
use ennio_core::lifecycle::{CleanupDetail, CleanupResult, SessionManager, SpawnRequest};
use ennio_core::paths;
use ennio_core::runtime::{Runtime, RuntimeCreateConfig, RuntimeHandle};
use ennio_core::session::{Session, SessionStatus};
use ennio_core::tracker::Issue;
use ennio_core::workspace::{Workspace, WorkspaceCreateConfig};
use ennio_db::repo::sessions;
use ennio_nats::EventPublisher;
use ennio_ssh::strategy::SshSessionStrategy;
use ennio_ssh::{RemoteNode, SshClient, SshRuntime, create_strategy};

use crate::event_bus::EventBus;
use crate::registry::PluginRegistry;

pub struct DefaultSessionManager {
    registry: Arc<PluginRegistry>,
    event_bus: Arc<EventBus>,
    config: Arc<OrchestratorConfig>,
    pool: SqlitePool,
    publisher: Arc<EventPublisher>,
    node_connections: RwLock<HashMap<String, Arc<Mutex<RemoteNode>>>>,
}

impl DefaultSessionManager {
    pub fn new(
        registry: Arc<PluginRegistry>,
        event_bus: Arc<EventBus>,
        config: Arc<OrchestratorConfig>,
        pool: SqlitePool,
        publisher: Arc<EventPublisher>,
    ) -> Self {
        Self {
            registry,
            event_bus,
            config,
            pool,
            publisher,
            node_connections: RwLock::new(HashMap::new()),
        }
    }

    fn find_project(&self, project_id: &ProjectId) -> Result<&ProjectConfig, EnnioError> {
        self.config.find_project(project_id)
    }

    fn resolve_runtime_name<'a>(&'a self, project: &'a ProjectConfig) -> &'a str {
        project
            .runtime
            .as_deref()
            .unwrap_or(self.config.defaults.runtime.as_str())
    }

    fn resolve_agent_name<'a>(&'a self, project: &'a ProjectConfig) -> &'a str {
        project
            .agent
            .as_deref()
            .unwrap_or(self.config.defaults.agent.as_str())
    }

    fn resolve_workspace_name<'a>(&'a self, project: &'a ProjectConfig) -> &'a str {
        project
            .workspace
            .as_deref()
            .unwrap_or(self.config.defaults.workspace.as_str())
    }

    fn generate_session_id(
        &self,
        project: &ProjectConfig,
        config_hash: &str,
    ) -> Result<SessionId, EnnioError> {
        let prefix = project
            .session_prefix
            .as_deref()
            .map(str::to_owned)
            .unwrap_or_else(|| paths::session_prefix_from_name(&project.name));

        let unique = &uuid::Uuid::new_v4().to_string()[..8];
        let id_str = format!("{config_hash}-{prefix}-{unique}");
        SessionId::new(id_str)
    }

    fn emit_event(
        &self,
        event_type: EventType,
        priority: EventPriority,
        session_id: &SessionId,
        project_id: &ProjectId,
        message: &str,
    ) {
        let ctx = crate::events::EventContext {
            event_bus: &self.event_bus,
            pool: &self.pool,
            publisher: &self.publisher,
        };
        crate::events::fire_event(&ctx, event_type, priority, session_id, project_id, message);
    }

    fn is_node_strategy(ssh_config: &SshConnectionConfig) -> bool {
        ssh_config.strategy == SshStrategyConfig::Node
    }

    async fn get_or_connect_node(
        &self,
        ssh_config: &SshConnectionConfig,
    ) -> Result<(), EnnioError> {
        let host = &ssh_config.host;
        {
            let connections = self.node_connections.read().await;
            if connections.contains_key(host) {
                return Ok(());
            }
        }

        let node_config = ssh_config
            .node_config
            .as_ref()
            .cloned() // clone: NodeConnectionConfig is small pure data
            .unwrap_or_default();

        let client = SshClient::connect(ssh_config).await?;

        let node = RemoteNode::connect(&client, &node_config, host)
            .await
            .map_err(|e| EnnioError::Node {
                host: host.to_owned(),
                message: e.to_string(),
            })?;

        let mut connections = self.node_connections.write().await;
        connections.insert(host.to_owned(), Arc::new(Mutex::new(node)));

        Ok(())
    }

    async fn build_ssh_client(
        ssh_config: &SshConnectionConfig,
    ) -> Result<(SshClient, Box<dyn SshSessionStrategy>), EnnioError> {
        let strategy = create_strategy(ssh_config.strategy);
        let client = SshClient::connect(ssh_config).await?;
        Ok((client, strategy))
    }

    fn build_remote_workspace(
        client: &SshClient,
        project: &ProjectConfig,
        workspace_name: &str,
    ) -> Result<Box<dyn Workspace>, EnnioError> {
        match workspace_name {
            "worktree" => Ok(Box::new(ennio_ssh::SshWorktreeWorkspace::new(
                client.clone(), // clone: SshClient uses Arc internally, cheap ref count bump
                project.path.to_string_lossy().into_owned(),
            ))),
            "clone" => Ok(Box::new(ennio_ssh::SshCloneWorkspace::new(
                client.clone(), // clone: SshClient uses Arc internally, cheap ref count bump
            ))),
            other => Err(EnnioError::Config {
                message: format!("unsupported remote workspace type: {other}"),
            }),
        }
    }

    async fn create_workspace(
        &self,
        project: &ProjectConfig,
        request: &SpawnRequest<'_>,
        session_id: &SessionId,
    ) -> Result<PathBuf, EnnioError> {
        let ws_config = WorkspaceCreateConfig {
            project_id: request.project_id,
            project,
            session_id,
            branch: request.branch,
        };

        info!(session_id = %session_id, "creating workspace");

        if let Some(ssh_config) = &project.ssh_config {
            if Self::is_node_strategy(ssh_config) {
                self.get_or_connect_node(ssh_config).await?;
                let workspace_name = self.resolve_workspace_name(project);
                let node_arc = {
                    let connections = self.node_connections.read().await;
                    Arc::clone(connections.get(&ssh_config.host).ok_or_else(|| {
                        EnnioError::Node {
                            host: ssh_config.host.clone(), // clone: building error with host context
                            message: "node connection lost".to_owned(),
                        }
                    })?)
                };
                let mut node = node_arc.lock().await;
                let workspace_path = node
                    .create_workspace(&ws_config, workspace_name)
                    .await
                    .map_err(|e| EnnioError::Node {
                        host: ssh_config.host.clone(), // clone: building error with host context
                        message: e.to_string(),
                    })?;
                return Ok(workspace_path);
            }

            let (client, _strategy) = Self::build_ssh_client(ssh_config).await?;
            let workspace_name = self.resolve_workspace_name(project);
            let remote_ws = Self::build_remote_workspace(&client, project, workspace_name)?;
            let workspace_path = remote_ws.create(&ws_config).await?;
            remote_ws.post_create(&workspace_path, &ws_config).await?;
            Ok(workspace_path)
        } else {
            let workspace_name = self.resolve_workspace_name(project);
            let workspace_plugin = self.registry.get_workspace(workspace_name)?;
            let workspace_path = workspace_plugin.create(&ws_config).await?;
            workspace_plugin
                .post_create(&workspace_path, &ws_config)
                .await?;
            Ok(workspace_path)
        }
    }

    async fn fetch_issue(
        &self,
        project: &ProjectConfig,
        request: &SpawnRequest<'_>,
    ) -> Result<Option<Issue>, EnnioError> {
        let Some(issue_id) = request.issue_id else {
            return Ok(None);
        };
        let Some(tracker_config) = &project.tracker_config else {
            return Ok(None);
        };
        let tracker = self.registry.get_tracker(&tracker_config.plugin)?;
        let issue = tracker.get_issue(request.project_id, issue_id).await?;
        Ok(Some(issue))
    }

    async fn create_runtime(
        &self,
        project: &ProjectConfig,
        session_id: &SessionId,
        workspace_path: &Path,
        launch_command: String,
        env: HashMap<String, String>,
        config_hash: &str,
    ) -> Result<(RuntimeHandle, String), EnnioError> {
        let prefix = project
            .session_prefix
            .as_deref()
            .map(str::to_owned)
            .unwrap_or_else(|| paths::session_prefix_from_name(&project.name));
        let tmux_session_name = paths::tmux_name(config_hash, &prefix, 0);

        let runtime_config = RuntimeCreateConfig {
            // clone: SessionId must be owned by RuntimeCreateConfig
            session_id: session_id.clone(),
            launch_command,
            env,
            cwd: workspace_path.to_string_lossy().into_owned(),
            // clone: tmux_session_name must be owned by RuntimeCreateConfig
            session_name: tmux_session_name.clone(),
        };

        info!(session_id = %session_id, "creating runtime");

        if let Some(ssh_config) = &project.ssh_config {
            if Self::is_node_strategy(ssh_config) {
                self.get_or_connect_node(ssh_config).await?;
                let node_arc = {
                    let connections = self.node_connections.read().await;
                    Arc::clone(connections.get(&ssh_config.host).ok_or_else(|| {
                        EnnioError::Node {
                            host: ssh_config.host.clone(), // clone: building error with host context
                            message: "node connection lost".to_owned(),
                        }
                    })?)
                };
                let mut node = node_arc.lock().await;
                let runtime_handle =
                    node.create_runtime(&runtime_config)
                        .await
                        .map_err(|e| EnnioError::Node {
                            host: ssh_config.host.clone(), // clone: building error with host context
                            message: e.to_string(),
                        })?;
                return Ok((runtime_handle, tmux_session_name));
            }

            let (client, strategy) = Self::build_ssh_client(ssh_config).await?;
            let ssh_runtime = SshRuntime::new(client, strategy);
            let runtime_handle = ssh_runtime.create(&runtime_config).await?;
            Ok((runtime_handle, tmux_session_name))
        } else {
            let runtime_name = self.resolve_runtime_name(project);
            let runtime_plugin = self.registry.get_runtime(runtime_name)?;
            let runtime_handle = runtime_plugin.create(&runtime_config).await?;
            Ok((runtime_handle, tmux_session_name))
        }
    }

    async fn kill_local(
        &self,
        session_id: &SessionId,
        session: &Session,
        project: &ProjectConfig,
    ) -> Result<(), EnnioError> {
        if let Some(ref handle) = session.runtime_handle {
            let runtime_name = self.resolve_runtime_name(project);
            let runtime_plugin = self.registry.get_runtime(runtime_name)?;
            if let Err(e) = runtime_plugin.destroy(handle).await {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "failed to destroy runtime, continuing with kill"
                );
            }
        }

        if let Some(ref ws_path) = session.workspace_path {
            let workspace_name = self.resolve_workspace_name(project);
            let workspace_plugin = self.registry.get_workspace(workspace_name)?;
            if let Err(e) = workspace_plugin.destroy(ws_path).await {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "failed to destroy workspace, continuing with kill"
                );
            }
        }

        Ok(())
    }

    async fn kill_remote(
        &self,
        session_id: &SessionId,
        session: &Session,
        project: &ProjectConfig,
    ) -> Result<(), EnnioError> {
        let ssh_config = project
            .ssh_config
            .as_ref()
            .ok_or_else(|| EnnioError::Config {
                message: "kill_remote called on non-remote project".to_owned(),
            })?;

        if Self::is_node_strategy(ssh_config) {
            return self
                .kill_remote_via_node(session_id, session, ssh_config)
                .await;
        }

        let (client, strategy) = Self::build_ssh_client(ssh_config).await?;

        if let Some(ref handle) = session.runtime_handle {
            let ssh_runtime = SshRuntime::new(
                client.clone(), // clone: SshClient uses Arc internally, cheap ref count bump
                strategy,
            );
            if let Err(e) = ssh_runtime.destroy(handle).await {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "failed to destroy remote runtime, continuing with kill"
                );
            }
        }

        if let Some(ref ws_path) = session.workspace_path {
            let workspace_name = self.resolve_workspace_name(project);
            let remote_ws = Self::build_remote_workspace(&client, project, workspace_name)?;
            if let Err(e) = remote_ws.destroy(ws_path).await {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "failed to destroy remote workspace, continuing with kill"
                );
            }
        }

        Ok(())
    }

    async fn kill_remote_via_node(
        &self,
        session_id: &SessionId,
        session: &Session,
        ssh_config: &SshConnectionConfig,
    ) -> Result<(), EnnioError> {
        self.get_or_connect_node(ssh_config).await?;
        let node_arc = {
            let connections = self.node_connections.read().await;
            Arc::clone(connections.get(&ssh_config.host).ok_or_else(|| {
                EnnioError::Node {
                    host: ssh_config.host.clone(), // clone: building error with host context
                    message: "node connection lost".to_owned(),
                }
            })?)
        };
        let mut node = node_arc.lock().await;

        if let Some(ref handle) = session.runtime_handle {
            if let Err(e) = node.destroy_runtime(handle).await {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "failed to destroy node runtime, continuing with kill"
                );
            }
        }

        if let Some(ref ws_path) = session.workspace_path {
            if let Err(e) = node.destroy_workspace(&ws_path.to_string_lossy()).await {
                warn!(
                    session_id = %session_id,
                    error = %e,
                    "failed to destroy node workspace, continuing with kill"
                );
            }
        }

        Ok(())
    }

    async fn should_cleanup(
        &self,
        session: &Session,
        project_id: &ProjectId,
    ) -> Result<bool, EnnioError> {
        match session.status {
            SessionStatus::Merged | SessionStatus::Done => Ok(true),
            _ => {
                let Some(issue_id) = session.issue_id.as_deref() else {
                    return Ok(false);
                };
                let project = self.find_project(project_id)?;
                let Some(tracker_config) = &project.tracker_config else {
                    return Ok(false);
                };
                let tracker = self.registry.get_tracker(&tracker_config.plugin)?;
                let issue = tracker.get_issue(project_id, issue_id).await?;
                Ok(tracker.is_completed(&issue).await.unwrap_or(false))
            }
        }
    }

    async fn cleanup_session(&self, session: &Session, project_id: &ProjectId) -> CleanupDetail {
        match self.kill(&session.id).await {
            Ok(()) => {
                self.emit_event(
                    EventType::SessionCleaned,
                    EventPriority::Info,
                    &session.id,
                    project_id,
                    "session cleaned up",
                );
                CleanupDetail {
                    // clone: SessionId must be owned by CleanupDetail
                    session_id: session.id.clone(),
                    success: true,
                    reason: "issue completed or PR merged".to_owned(),
                }
            }
            Err(e) => {
                warn!(
                    session_id = %session.id,
                    error = %e,
                    "cleanup failed for session"
                );
                CleanupDetail {
                    // clone: SessionId must be owned by CleanupDetail
                    session_id: session.id.clone(),
                    success: false,
                    reason: e.to_string(),
                }
            }
        }
    }

    async fn restore_workspace(
        &self,
        session: &Session,
        project: &ProjectConfig,
        session_id: &SessionId,
    ) -> Result<(), EnnioError> {
        let Some(ref ws_path) = session.workspace_path else {
            return Ok(());
        };

        let ws_config = WorkspaceCreateConfig {
            project_id: &session.project_id,
            project,
            session_id,
            branch: session.branch.as_deref(),
        };

        if let Some(ssh_config) = &project.ssh_config {
            let (client, _strategy) = Self::build_ssh_client(ssh_config).await?;
            let workspace_name = self.resolve_workspace_name(project);
            let remote_ws = Self::build_remote_workspace(&client, project, workspace_name)?;
            remote_ws.restore(ws_path, &ws_config).await?;
        } else {
            let workspace_name = self.resolve_workspace_name(project);
            let workspace_plugin = self.registry.get_workspace(workspace_name)?;
            workspace_plugin.restore(ws_path, &ws_config).await?;
        }

        Ok(())
    }

    async fn restore_runtime(
        &self,
        session: &Session,
        project: &ProjectConfig,
        session_id: &SessionId,
    ) -> Result<RuntimeHandle, EnnioError> {
        let agent_name = self.resolve_agent_name(project);
        let agent_plugin = self.registry.get_agent(agent_name)?;

        let restore_command = agent_plugin.get_restore_command(session, project).await?;

        let cwd = session
            .workspace_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        let runtime_config = RuntimeCreateConfig {
            // clone: SessionId must be owned by RuntimeCreateConfig
            session_id: session_id.clone(),
            launch_command: restore_command.unwrap_or_default(),
            env: HashMap::new(),
            cwd,
            session_name: session
                .tmux_name
                // clone: tmux_name must be owned by RuntimeCreateConfig
                .clone()
                .unwrap_or_else(|| session_id.to_string()),
        };

        if let Some(ssh_config) = &project.ssh_config {
            let (client, strategy) = Self::build_ssh_client(ssh_config).await?;
            let ssh_runtime = SshRuntime::new(client, strategy);
            Ok(ssh_runtime.create(&runtime_config).await?)
        } else {
            let runtime_name = self.resolve_runtime_name(project);
            let runtime_plugin = self.registry.get_runtime(runtime_name)?;
            Ok(runtime_plugin.create(&runtime_config).await?)
        }
    }
}

#[async_trait]
impl SessionManager for DefaultSessionManager {
    async fn spawn(&self, request: &SpawnRequest<'_>) -> Result<Session, EnnioError> {
        let project = self.find_project(request.project_id)?;
        let config_hash = paths::config_hash(&project.path.to_string_lossy());
        let session_id = self.generate_session_id(project, &config_hash)?;

        let workspace_path = self.create_workspace(project, request, &session_id).await?;
        let issue = self.fetch_issue(project, request).await?;

        let agent_name = self.resolve_agent_name(project);
        let agent_plugin = self.registry.get_agent(agent_name)?;
        let agent_launch_config = AgentLaunchConfig {
            session_id: &session_id,
            project_config: project,
            issue: issue.as_ref(),
            prompt: request.prompt,
            permissions: project
                .agent_config
                .as_ref()
                .and_then(|c| c.permissions.as_deref()),
            model: project
                .agent_config
                .as_ref()
                .and_then(|c| c.model.as_deref()),
            system_prompt: None,
            system_prompt_file: None,
        };

        let launch_command = agent_plugin.get_launch_command(&agent_launch_config);
        let env = agent_plugin.get_environment(&agent_launch_config);

        if project.is_remote() {
            warn!("skipping workspace hooks for remote project — hooks write local files");
        } else {
            let data_dir = paths::data_dir(&config_hash, request.project_id.as_str())?;
            let hooks_config = WorkspaceHooksConfig {
                session_id: &session_id,
                data_dir: &data_dir,
                project_config: project,
            };
            agent_plugin
                .setup_workspace_hooks(&workspace_path, &hooks_config)
                .await?;
        }

        let (runtime_handle, tmux_session_name) = self
            .create_runtime(
                project,
                &session_id,
                &workspace_path,
                launch_command,
                env,
                &config_hash,
            )
            .await?;

        let session = Session {
            id: session_id,
            // clone: ProjectId must be owned by the Session struct
            project_id: request.project_id.clone(),
            status: SessionStatus::Spawning,
            activity: None,
            branch: request.branch.map(str::to_owned),
            issue_id: request.issue_id.map(str::to_owned),
            workspace_path: Some(workspace_path),
            runtime_handle: Some(runtime_handle),
            agent_info: None,
            agent_name: Some(agent_name.to_owned()),
            pr_url: None,
            pr_number: None,
            tmux_name: Some(tmux_session_name),
            config_hash,
            role: request.role.map(str::to_owned),
            metadata: HashMap::new(),
            created_at: Utc::now(),
            last_activity_at: Utc::now(),
            restored_at: None,
            archived_at: None,
        };

        if agent_plugin.prompt_delivery() == PromptDelivery::PostLaunch {
            agent_plugin.post_launch_setup(&session).await?;
        }

        sessions::insert(&self.pool, &session)
            .await
            .map_err(|e| EnnioError::Database {
                message: e.to_string(),
            })?;

        self.emit_event(
            EventType::SessionSpawned,
            EventPriority::Info,
            &session.id,
            &session.project_id,
            "session spawned",
        );

        info!(session_id = %session.id, "session spawned");
        Ok(session)
    }

    async fn restore(&self, session_id: &SessionId) -> Result<Session, EnnioError> {
        let session = self.get(session_id).await?;

        if !session.status.is_restorable() {
            return Err(EnnioError::Session {
                // clone: SessionId must be owned by the error variant
                session_id: session_id.clone(),
                message: format!("session is in {} state, not restorable", session.status),
            });
        }

        let project = self.find_project(&session.project_id)?;

        self.restore_workspace(&session, project, session_id)
            .await?;

        let runtime_handle = self.restore_runtime(&session, project, session_id).await?;

        sessions::update_status(&self.pool, session_id, SessionStatus::Working)
            .await
            .map_err(|e| EnnioError::Database {
                message: e.to_string(),
            })?;

        self.emit_event(
            EventType::SessionRestored,
            EventPriority::Info,
            session_id,
            &session.project_id,
            "session restored",
        );

        let mut restored = session;
        restored.status = SessionStatus::Working;
        restored.runtime_handle = Some(runtime_handle);
        restored.restored_at = Some(Utc::now());
        restored.last_activity_at = Utc::now();

        info!(session_id = %session_id, "session restored");
        Ok(restored)
    }

    async fn list(&self, project_id: Option<&ProjectId>) -> Result<Vec<Session>, EnnioError> {
        sessions::list(&self.pool, project_id)
            .await
            .map_err(|e| EnnioError::Database {
                message: e.to_string(),
            })
    }

    async fn get(&self, session_id: &SessionId) -> Result<Session, EnnioError> {
        sessions::get(&self.pool, session_id)
            .await
            .map_err(|e| EnnioError::Database {
                message: e.to_string(),
            })?
            .ok_or_else(|| EnnioError::NotFound {
                entity: "session".to_owned(),
                id: session_id.to_string(),
            })
    }

    async fn kill(&self, session_id: &SessionId) -> Result<(), EnnioError> {
        let session = self.get(session_id).await?;

        if session.status.is_terminal() {
            debug!(session_id = %session_id, "session already in terminal state");
            return Ok(());
        }

        let project = self.find_project(&session.project_id)?;

        if project.is_remote() {
            self.kill_remote(session_id, &session, project).await?;
        } else {
            self.kill_local(session_id, &session, project).await?;
        }

        sessions::update_status(&self.pool, session_id, SessionStatus::Killed)
            .await
            .map_err(|e| EnnioError::Database {
                message: e.to_string(),
            })?;

        self.emit_event(
            EventType::SessionKilled,
            EventPriority::Info,
            session_id,
            &session.project_id,
            "session killed",
        );

        info!(session_id = %session_id, "session killed");
        Ok(())
    }

    async fn cleanup(&self, project_id: &ProjectId) -> Result<CleanupResult, EnnioError> {
        let all_sessions = self.list(Some(project_id)).await?;

        let mut sessions_cleaned = 0u32;
        let mut sessions_failed = 0u32;
        let mut details = Vec::new();

        for session in &all_sessions {
            if session.status.is_terminal() {
                continue;
            }

            if !self.should_cleanup(session, project_id).await? {
                continue;
            }

            let detail = self.cleanup_session(session, project_id).await;
            match detail.success {
                true => sessions_cleaned = sessions_cleaned.saturating_add(1),
                false => sessions_failed = sessions_failed.saturating_add(1),
            }
            details.push(detail);
        }

        info!(
            project_id = %project_id,
            cleaned = sessions_cleaned,
            failed = sessions_failed,
            "cleanup complete"
        );

        Ok(CleanupResult {
            sessions_cleaned,
            sessions_failed,
            details,
        })
    }

    async fn send(&self, session_id: &SessionId, message: &str) -> Result<(), EnnioError> {
        let session = self.get(session_id).await?;

        let handle = session.runtime_handle.as_ref().ok_or_else(|| {
            EnnioError::Session {
                // clone: SessionId must be owned by the error variant
                session_id: session_id.clone(),
                message: "session has no runtime handle".to_owned(),
            }
        })?;

        let project = self.find_project(&session.project_id)?;

        if let Some(ssh_config) = &project.ssh_config {
            if Self::is_node_strategy(ssh_config) {
                self.get_or_connect_node(ssh_config).await?;
                let node_arc = {
                    let connections = self.node_connections.read().await;
                    Arc::clone(connections.get(&ssh_config.host).ok_or_else(|| {
                        EnnioError::Node {
                            host: ssh_config.host.clone(), // clone: building error with host context
                            message: "node connection lost".to_owned(),
                        }
                    })?)
                };
                let mut node = node_arc.lock().await;
                node.send_message(handle, message)
                    .await
                    .map_err(|e| EnnioError::Node {
                        host: ssh_config.host.clone(), // clone: building error with host context
                        message: e.to_string(),
                    })?;
            } else {
                let (client, strategy) = Self::build_ssh_client(ssh_config).await?;
                let ssh_runtime = SshRuntime::new(client, strategy);
                ssh_runtime.send_message(handle, message).await?;
            }
        } else {
            let runtime_name = self.resolve_runtime_name(project);
            let runtime_plugin = self.registry.get_runtime(runtime_name)?;
            runtime_plugin.send_message(handle, message).await?;
        }

        debug!(session_id = %session_id, "message sent to session");
        Ok(())
    }
}
