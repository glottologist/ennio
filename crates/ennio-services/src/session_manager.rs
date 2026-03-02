use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use ennio_core::agent::{AgentLaunchConfig, PromptDelivery, WorkspaceHooksConfig};
use ennio_core::config::{OrchestratorConfig, ProjectConfig};
use ennio_core::error::EnnioError;
use ennio_core::event::{EventPriority, EventType, OrchestratorEvent};
use ennio_core::id::{EventId, ProjectId, SessionId};
use ennio_core::lifecycle::{CleanupDetail, CleanupResult, SessionManager, SpawnRequest};
use ennio_core::paths;
use ennio_core::runtime::{RuntimeCreateConfig, RuntimeHandle};
use ennio_core::session::{Session, SessionStatus};
use ennio_core::tracker::Issue;
use ennio_core::workspace::WorkspaceCreateConfig;
use ennio_db::repo::{events, sessions};
use ennio_nats::EventPublisher;

use crate::event_bus::EventBus;
use crate::registry::PluginRegistry;

pub struct DefaultSessionManager {
    registry: Arc<PluginRegistry>,
    event_bus: Arc<EventBus>,
    config: Arc<OrchestratorConfig>,
    pool: PgPool,
    publisher: Arc<EventPublisher>,
}

impl DefaultSessionManager {
    pub fn new(
        registry: Arc<PluginRegistry>,
        event_bus: Arc<EventBus>,
        config: Arc<OrchestratorConfig>,
        pool: PgPool,
        publisher: Arc<EventPublisher>,
    ) -> Self {
        Self {
            registry,
            event_bus,
            config,
            pool,
            publisher,
        }
    }

    fn find_project(&self, project_id: &ProjectId) -> Result<&ProjectConfig, EnnioError> {
        self.config
            .projects
            .iter()
            .find(|p| p.project_id.as_ref().is_some_and(|id| id == project_id))
            .ok_or_else(|| EnnioError::NotFound {
                entity: "project".to_owned(),
                id: project_id.to_string(),
            })
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
        let event = OrchestratorEvent {
            id: EventId::random(),
            event_type,
            priority,
            // clone: SessionId must be owned by the event struct, caller retains its reference
            session_id: session_id.clone(),
            // clone: ProjectId must be owned by the event struct, caller retains its reference
            project_id: project_id.clone(),
            timestamp: Utc::now(),
            message: message.to_owned(),
            data: serde_json::Value::Null,
        };

        self.event_bus.publish(&event);

        // clone: Arc reference count increment for async task ownership
        let publisher = Arc::clone(&self.publisher);
        // clone: event must be owned by the spawned task
        let event_for_nats = event.clone();
        // clone: PgPool uses Arc internally, this is a cheap reference count increment
        let pool = self.pool.clone();
        tokio::spawn(async move {
            if let Err(e) = publisher.publish_event(&event_for_nats).await {
                warn!("failed to publish event to NATS: {e}");
            }
            if let Err(e) = events::insert(&pool, &event_for_nats).await {
                error!("failed to persist event: {e}");
            }
        });
    }

    async fn create_workspace(
        &self,
        project: &ProjectConfig,
        request: &SpawnRequest<'_>,
        session_id: &SessionId,
    ) -> Result<PathBuf, EnnioError> {
        let workspace_name = self.resolve_workspace_name(project);
        let workspace_plugin = self.registry.get_workspace(workspace_name)?;
        let ws_config = WorkspaceCreateConfig {
            project_id: request.project_id,
            project,
            session_id,
            branch: request.branch,
        };

        info!(session_id = %session_id, "creating workspace");
        let workspace_path = workspace_plugin.create(&ws_config).await?;
        workspace_plugin
            .post_create(&workspace_path, &ws_config)
            .await?;
        Ok(workspace_path)
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

        let runtime_name = self.resolve_runtime_name(project);
        let runtime_plugin = self.registry.get_runtime(runtime_name)?;
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
        let runtime_handle = runtime_plugin.create(&runtime_config).await?;
        Ok((runtime_handle, tmux_session_name))
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

        let data_dir = paths::data_dir(&config_hash, request.project_id.as_str());
        let hooks_config = WorkspaceHooksConfig {
            session_id: &session_id,
            data_dir: &data_dir,
            project_config: project,
        };
        agent_plugin
            .setup_workspace_hooks(&workspace_path, &hooks_config)
            .await?;

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
        let session = sessions::get(&self.pool, session_id)
            .await
            .map_err(|e| EnnioError::Database {
                message: e.to_string(),
            })?
            .ok_or_else(|| EnnioError::NotFound {
                entity: "session".to_owned(),
                id: session_id.to_string(),
            })?;

        if !session.status.is_restorable() {
            return Err(EnnioError::Session {
                // clone: SessionId must be owned by the error variant
                session_id: session_id.clone(),
                message: format!("session is in {} state, not restorable", session.status),
            });
        }

        let project = self.find_project(&session.project_id)?;

        let workspace_name = self.resolve_workspace_name(project);
        let workspace_plugin = self.registry.get_workspace(workspace_name)?;

        if let Some(ref ws_path) = session.workspace_path {
            let ws_config = WorkspaceCreateConfig {
                project_id: &session.project_id,
                project,
                session_id,
                branch: session.branch.as_deref(),
            };
            workspace_plugin.restore(ws_path, &ws_config).await?;
        }

        let agent_name = self.resolve_agent_name(project);
        let agent_plugin = self.registry.get_agent(agent_name)?;

        let restore_command = agent_plugin.get_restore_command(&session, project).await?;

        let runtime_name = self.resolve_runtime_name(project);
        let runtime_plugin = self.registry.get_runtime(runtime_name)?;

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

        let runtime_handle = runtime_plugin.create(&runtime_config).await?;

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

            let should_clean = match session.status {
                SessionStatus::Merged | SessionStatus::Done => true,
                _ => {
                    if let Some(issue_id) = session.issue_id.as_deref() {
                        let project = self.find_project(project_id)?;
                        if let Some(tracker_config) = &project.tracker_config {
                            let tracker = self.registry.get_tracker(&tracker_config.plugin)?;
                            let issue = tracker.get_issue(project_id, issue_id).await?;
                            tracker.is_completed(&issue).await.unwrap_or(false)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            };

            if !should_clean {
                continue;
            }

            match self.kill(&session.id).await {
                Ok(()) => {
                    sessions_cleaned = sessions_cleaned.saturating_add(1);
                    details.push(CleanupDetail {
                        // clone: SessionId must be owned by CleanupDetail
                        session_id: session.id.clone(),
                        success: true,
                        reason: "issue completed or PR merged".to_owned(),
                    });

                    self.emit_event(
                        EventType::SessionCleaned,
                        EventPriority::Info,
                        &session.id,
                        project_id,
                        "session cleaned up",
                    );
                }
                Err(e) => {
                    sessions_failed = sessions_failed.saturating_add(1);
                    details.push(CleanupDetail {
                        // clone: SessionId must be owned by CleanupDetail
                        session_id: session.id.clone(),
                        success: false,
                        reason: e.to_string(),
                    });
                    warn!(
                        session_id = %session.id,
                        error = %e,
                        "cleanup failed for session"
                    );
                }
            }
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
        let runtime_name = self.resolve_runtime_name(project);
        let runtime_plugin = self.registry.get_runtime(runtime_name)?;

        runtime_plugin.send_message(handle, message).await?;

        debug!(session_id = %session_id, "message sent to session");
        Ok(())
    }
}
