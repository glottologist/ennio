use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use ennio_core::config::OrchestratorConfig;
use ennio_core::error::EnnioError;
use ennio_core::event::{EventPriority, EventType, OrchestratorEvent};
use ennio_core::id::{EventId, ProjectId, SessionId};
use ennio_core::lifecycle::{LifecycleManager, SessionManager, SessionState};
use ennio_core::reaction::ReactionConfig;
use ennio_core::scm::{CIStatus, PRState, ReviewDecision};
use ennio_core::session::{Session, SessionStatus};
use ennio_db::repo::{events, sessions};
use ennio_nats::EventPublisher;

use crate::event_bus::EventBus;
use crate::registry::PluginRegistry;

struct ReactionTracker {
    attempt_counts: HashMap<(SessionId, String), u32>,
}

impl ReactionTracker {
    fn new() -> Self {
        Self {
            attempt_counts: HashMap::new(),
        }
    }

    fn get_attempts(&self, session_id: &SessionId, reaction_key: &str) -> u32 {
        self.attempt_counts
            .get(&(session_id.clone(), reaction_key.to_owned()))
            .copied()
            .unwrap_or(0)
    }

    fn increment_attempts(&mut self, session_id: &SessionId, reaction_key: &str) {
        let key = (session_id.clone(), reaction_key.to_owned());
        let count = self.attempt_counts.entry(key).or_insert(0);
        *count = count.saturating_add(1);
    }

    fn clear_session(&mut self, session_id: &SessionId) {
        self.attempt_counts.retain(|(sid, _), _| sid != session_id);
    }
}

pub struct DefaultLifecycleManager {
    registry: Arc<PluginRegistry>,
    event_bus: Arc<EventBus>,
    config: Arc<OrchestratorConfig>,
    pool: PgPool,
    publisher: Arc<EventPublisher>,
    session_manager: Arc<dyn SessionManager>,
    states: RwLock<HashMap<SessionId, SessionState>>,
    reaction_tracker: RwLock<ReactionTracker>,
    running: RwLock<bool>,
}

impl DefaultLifecycleManager {
    pub fn new(
        registry: Arc<PluginRegistry>,
        event_bus: Arc<EventBus>,
        config: Arc<OrchestratorConfig>,
        pool: PgPool,
        publisher: Arc<EventPublisher>,
        session_manager: Arc<dyn SessionManager>,
    ) -> Self {
        Self {
            registry,
            event_bus,
            config,
            pool,
            publisher,
            session_manager,
            states: RwLock::new(HashMap::new()),
            reaction_tracker: RwLock::new(ReactionTracker::new()),
            running: RwLock::new(false),
        }
    }

    pub async fn poll_sessions(&self) -> Result<(), EnnioError> {
        let all_sessions =
            sessions::list(&self.pool, None)
                .await
                .map_err(|e| EnnioError::Database {
                    message: e.to_string(),
                })?;

        for session in &all_sessions {
            if session.status.is_terminal() {
                continue;
            }

            match self.check_session_status(session).await {
                Ok(new_state) => {
                    let mut states = self.states.write().await;
                    // clone: SessionId must be owned as HashMap key
                    states.insert(session.id.clone(), new_state);
                }
                Err(e) => {
                    warn!(
                        session_id = %session.id,
                        error = %e,
                        "failed to check session status"
                    );
                }
            }
        }

        Ok(())
    }

    async fn check_session_status(&self, session: &Session) -> Result<SessionState, EnnioError> {
        let project = self
            .config
            .projects
            .iter()
            .find(|p| {
                p.project_id
                    .as_ref()
                    .is_some_and(|id| id == &session.project_id)
            })
            .ok_or_else(|| EnnioError::NotFound {
                entity: "project".to_owned(),
                id: session.project_id.to_string(),
            })?;

        let runtime_name = project
            .runtime
            .as_deref()
            .unwrap_or(self.config.defaults.runtime.as_str());

        let runtime_alive = if let Some(ref handle) = session.runtime_handle {
            let runtime = self.registry.get_runtime(runtime_name)?;
            runtime.is_alive(handle).await.unwrap_or(false)
        } else {
            false
        };

        if !runtime_alive && !session.status.is_terminal() {
            let new_status = SessionStatus::Exited;
            self.transition_status(session, new_status).await?;
            return Ok(SessionState {
                // clone: SessionId must be owned by SessionState
                session_id: session.id.clone(),
                status: new_status,
                last_checked: Utc::now(),
            });
        }

        let agent_name = project
            .agent
            .as_deref()
            .unwrap_or(self.config.defaults.agent.as_str());

        let agent_plugin = self.registry.get_agent(agent_name)?;

        if let Some(ref handle) = session.runtime_handle {
            let process_running = agent_plugin.is_process_running(handle).await?;
            if !process_running {
                self.transition_status(session, SessionStatus::Exited)
                    .await?;
                return Ok(SessionState {
                    // clone: SessionId must be owned by SessionState
                    session_id: session.id.clone(),
                    status: SessionStatus::Exited,
                    last_checked: Utc::now(),
                });
            }
        }

        let new_status = self.determine_status_from_external(session, project).await;

        if new_status != session.status {
            self.transition_status(session, new_status).await?;
            self.handle_reactions(session, new_status).await;
        }

        Ok(SessionState {
            // clone: SessionId must be owned by SessionState
            session_id: session.id.clone(),
            status: new_status,
            last_checked: Utc::now(),
        })
    }

    async fn determine_status_from_external(
        &self,
        session: &Session,
        project: &ennio_core::config::ProjectConfig,
    ) -> SessionStatus {
        let pr_number = match session.pr_number {
            Some(n) => n,
            None => {
                if let Some(branch) = session.branch.as_deref() {
                    if let Some(scm_config) = &project.scm_config {
                        if let Ok(scm) = self.registry.get_scm(&scm_config.plugin) {
                            match scm.detect_pr(&session.project_id, branch).await {
                                Ok(Some(pr)) => pr.number,
                                _ => return session.status,
                            }
                        } else {
                            return session.status;
                        }
                    } else {
                        return session.status;
                    }
                } else {
                    return session.status;
                }
            }
        };

        let scm_config = match &project.scm_config {
            Some(c) => c,
            None => return session.status,
        };

        let scm = match self.registry.get_scm(&scm_config.plugin) {
            Ok(s) => s,
            Err(_) => return session.status,
        };

        let pr_state = scm
            .get_pr_state(&session.project_id, pr_number)
            .await
            .unwrap_or(PRState::Open);

        if pr_state == PRState::Merged {
            return SessionStatus::Merged;
        }

        let ci_status = scm
            .get_ci_summary(&session.project_id, pr_number)
            .await
            .unwrap_or(CIStatus::Pending);

        let review_decision = scm
            .get_review_decision(&session.project_id, pr_number)
            .await
            .unwrap_or(ReviewDecision::Pending);

        let mergeability = scm
            .get_mergeability(&session.project_id, pr_number)
            .await
            .ok();

        let has_conflicts = mergeability.as_ref().is_some_and(|m| !m.no_conflicts);

        if has_conflicts {
            return SessionStatus::MergeConflicts;
        }

        match (ci_status, review_decision) {
            (CIStatus::Failing, _) => {
                if session.status == SessionStatus::CiFixSent {
                    SessionStatus::CiFixFailed
                } else {
                    SessionStatus::CiFailed
                }
            }
            (CIStatus::Passing, ReviewDecision::Approved) => SessionStatus::Approved,
            (CIStatus::Passing, ReviewDecision::ChangesRequested) => {
                SessionStatus::ChangesRequested
            }
            (CIStatus::Passing, ReviewDecision::Pending) => SessionStatus::ReviewPending,
            (CIStatus::Passing, _) => SessionStatus::CiPassing,
            (_, ReviewDecision::ChangesRequested) => SessionStatus::ChangesRequested,
            _ => {
                if pr_state == PRState::Draft {
                    SessionStatus::PrDraft
                } else {
                    SessionStatus::PrOpen
                }
            }
        }
    }

    async fn transition_status(
        &self,
        session: &Session,
        new_status: SessionStatus,
    ) -> Result<(), EnnioError> {
        debug!(
            session_id = %session.id,
            old_status = %session.status,
            new_status = %new_status,
            "status transition"
        );

        sessions::update_status(&self.pool, &session.id, new_status)
            .await
            .map_err(|e| EnnioError::Database {
                message: e.to_string(),
            })?;

        let event_type = status_to_event_type(new_status);
        self.emit_event(
            event_type,
            EventPriority::Info,
            &session.id,
            &session.project_id,
            &format!("status changed to {new_status}"),
        );

        if new_status.is_terminal() {
            let mut tracker = self.reaction_tracker.write().await;
            tracker.clear_session(&session.id);
        }

        Ok(())
    }

    async fn handle_reactions(&self, session: &Session, new_status: SessionStatus) {
        let reaction_key = match new_status {
            SessionStatus::CiFailed => "ci-failed",
            SessionStatus::ChangesRequested => "changes-requested",
            SessionStatus::MergeConflicts => "merge-conflicts",
            SessionStatus::Approved => "approved-and-green",
            SessionStatus::Exited => "agent-exited",
            _ => return,
        };

        let merged_reactions = self.merge_reactions(session);

        let reaction_config = match merged_reactions.get(reaction_key) {
            Some(config) if config.enabled => config,
            _ => return,
        };

        let tracker = self.reaction_tracker.read().await;
        let attempts = tracker.get_attempts(&session.id, reaction_key);
        drop(tracker);

        if reaction_config.retries > 0 && attempts > reaction_config.retries {
            self.emit_event(
                EventType::ReactionEscalated,
                EventPriority::Urgent,
                &session.id,
                &session.project_id,
                &format!(
                    "reaction '{reaction_key}' exhausted retries ({attempts}/{})",
                    reaction_config.retries
                ),
            );
            return;
        }

        match reaction_config.action {
            ennio_core::reaction::ReactionAction::SendToAgent => {
                let message = reaction_config
                    .message
                    .as_deref()
                    .unwrap_or(default_reaction_message(reaction_key));

                match self.session_manager.send(&session.id, message).await {
                    Ok(()) => {
                        info!(
                            session_id = %session.id,
                            reaction = reaction_key,
                            "reaction sent to agent"
                        );
                    }
                    Err(e) => {
                        error!(
                            session_id = %session.id,
                            reaction = reaction_key,
                            error = %e,
                            "failed to send reaction to agent"
                        );
                    }
                }
            }
            ennio_core::reaction::ReactionAction::Notify => {
                self.emit_event(
                    EventType::ReactionTriggered,
                    reaction_config.priority,
                    &session.id,
                    &session.project_id,
                    &format!("notification reaction triggered: {reaction_key}"),
                );
            }
            ennio_core::reaction::ReactionAction::AutoMerge => {
                info!(
                    session_id = %session.id,
                    "auto-merge reaction triggered"
                );
                // TODO: implement auto-merge via SCM plugin
            }
        }

        let mut tracker = self.reaction_tracker.write().await;
        tracker.increment_attempts(&session.id, reaction_key);

        self.emit_event(
            EventType::ReactionTriggered,
            EventPriority::Info,
            &session.id,
            &session.project_id,
            &format!("reaction '{reaction_key}' executed (attempt {attempts})"),
        );
    }

    fn merge_reactions(&self, session: &Session) -> HashMap<String, ReactionConfig> {
        // clone: global reactions must be owned so project overrides can be merged in
        let mut merged = self.config.reactions.clone();

        let project = self.config.projects.iter().find(|p| {
            p.project_id
                .as_ref()
                .is_some_and(|id| id == &session.project_id)
        });

        if let Some(project) = project {
            for (key, config) in &project.reactions {
                // clone: ReactionConfig must be owned as the HashMap value
                merged.insert(key.to_owned(), config.clone());
            }
        }

        merged
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
            // clone: SessionId must be owned by the event struct
            session_id: session_id.clone(),
            // clone: ProjectId must be owned by the event struct
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
                warn!("failed to publish lifecycle event to NATS: {e}");
            }
            if let Err(e) = events::insert(&pool, &event_for_nats).await {
                error!("failed to persist lifecycle event: {e}");
            }
        });
    }
}

fn status_to_event_type(status: SessionStatus) -> EventType {
    match status {
        SessionStatus::Spawning => EventType::SessionSpawned,
        SessionStatus::Working => EventType::SessionWorking,
        SessionStatus::PrOpen | SessionStatus::PrDraft => EventType::PrCreated,
        SessionStatus::CiPassing => EventType::CiPassing,
        SessionStatus::CiFailed => EventType::CiFailing,
        SessionStatus::CiFixSent => EventType::CiFixSent,
        SessionStatus::CiFixFailed => EventType::CiFixFailed,
        SessionStatus::ReviewPending => EventType::ReviewPending,
        SessionStatus::ChangesRequested => EventType::ReviewChangesRequested,
        SessionStatus::Approved => EventType::ReviewApproved,
        SessionStatus::MergeConflicts => EventType::MergeConflicts,
        SessionStatus::Merged => EventType::PrMerged,
        SessionStatus::Done => EventType::MergeCompleted,
        SessionStatus::Exited => EventType::SessionExited,
        SessionStatus::Killed => EventType::SessionKilled,
    }
}

fn default_reaction_message(reaction_key: &str) -> &str {
    match reaction_key {
        "ci-failed" => "CI is failing. Please investigate the failures and fix them.",
        "changes-requested" => {
            "Changes have been requested on your PR. Please review the comments and address them."
        }
        "merge-conflicts" => "There are merge conflicts. Please resolve them.",
        "bugbot-comments" => {
            "There are new review comments from automated tools. Please review and address them."
        }
        _ => "Please check the current state of the PR and take appropriate action.",
    }
}

#[async_trait]
impl LifecycleManager for DefaultLifecycleManager {
    async fn start(&self) -> Result<(), EnnioError> {
        let mut running = self.running.write().await;
        if *running {
            return Err(EnnioError::Internal {
                message: "lifecycle manager already running".to_owned(),
            });
        }
        *running = true;
        drop(running);

        info!("lifecycle manager started");
        Ok(())
    }

    async fn stop(&self) -> Result<(), EnnioError> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(EnnioError::Internal {
                message: "lifecycle manager not running".to_owned(),
            });
        }
        *running = false;
        drop(running);

        info!("lifecycle manager stopped");
        Ok(())
    }

    async fn get_states(&self) -> Result<HashMap<SessionId, SessionState>, EnnioError> {
        let states = self.states.read().await;
        // clone: HashMap contents must be owned by the caller
        Ok(states.clone())
    }

    async fn check(&self, session_id: &SessionId) -> Result<SessionState, EnnioError> {
        let session = self.session_manager.get(session_id).await?;

        let state = self.check_session_status(&session).await?;

        let mut states = self.states.write().await;
        // clone: SessionState must be owned as HashMap value, returning original to caller
        states.insert(session_id.clone(), state.clone());

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(SessionStatus::Spawning, EventType::SessionSpawned)]
    #[case(SessionStatus::Working, EventType::SessionWorking)]
    #[case(SessionStatus::PrOpen, EventType::PrCreated)]
    #[case(SessionStatus::PrDraft, EventType::PrCreated)]
    #[case(SessionStatus::CiPassing, EventType::CiPassing)]
    #[case(SessionStatus::CiFailed, EventType::CiFailing)]
    #[case(SessionStatus::CiFixSent, EventType::CiFixSent)]
    #[case(SessionStatus::CiFixFailed, EventType::CiFixFailed)]
    #[case(SessionStatus::ReviewPending, EventType::ReviewPending)]
    #[case(SessionStatus::ChangesRequested, EventType::ReviewChangesRequested)]
    #[case(SessionStatus::Approved, EventType::ReviewApproved)]
    #[case(SessionStatus::MergeConflicts, EventType::MergeConflicts)]
    #[case(SessionStatus::Merged, EventType::PrMerged)]
    #[case(SessionStatus::Done, EventType::MergeCompleted)]
    #[case(SessionStatus::Exited, EventType::SessionExited)]
    #[case(SessionStatus::Killed, EventType::SessionKilled)]
    fn status_maps_to_event_type(#[case] status: SessionStatus, #[case] expected: EventType) {
        assert_eq!(status_to_event_type(status), expected);
    }

    #[rstest]
    #[case("ci-failed")]
    #[case("changes-requested")]
    #[case("merge-conflicts")]
    #[case("bugbot-comments")]
    #[case("unknown-key")]
    fn default_reaction_messages_non_empty(#[case] key: &str) {
        let msg = default_reaction_message(key);
        assert!(!msg.is_empty());
    }

    #[test]
    fn reaction_tracker_increments() {
        let mut tracker = ReactionTracker::new();
        let sid = SessionId::new("test-session").unwrap();

        assert_eq!(tracker.get_attempts(&sid, "ci-failed"), 0);

        tracker.increment_attempts(&sid, "ci-failed");
        assert_eq!(tracker.get_attempts(&sid, "ci-failed"), 1);

        tracker.increment_attempts(&sid, "ci-failed");
        assert_eq!(tracker.get_attempts(&sid, "ci-failed"), 2);
    }

    #[test]
    fn reaction_tracker_clears_session() {
        let mut tracker = ReactionTracker::new();
        let sid = SessionId::new("test-session").unwrap();

        tracker.increment_attempts(&sid, "ci-failed");
        tracker.increment_attempts(&sid, "changes-requested");

        tracker.clear_session(&sid);

        assert_eq!(tracker.get_attempts(&sid, "ci-failed"), 0);
        assert_eq!(tracker.get_attempts(&sid, "changes-requested"), 0);
    }

    #[test]
    fn reaction_tracker_isolates_sessions() {
        let mut tracker = ReactionTracker::new();
        let sid1 = SessionId::new("session-1").unwrap();
        let sid2 = SessionId::new("session-2").unwrap();

        tracker.increment_attempts(&sid1, "ci-failed");
        tracker.increment_attempts(&sid2, "ci-failed");

        assert_eq!(tracker.get_attempts(&sid1, "ci-failed"), 1);
        assert_eq!(tracker.get_attempts(&sid2, "ci-failed"), 1);

        tracker.clear_session(&sid1);
        assert_eq!(tracker.get_attempts(&sid1, "ci-failed"), 0);
        assert_eq!(tracker.get_attempts(&sid2, "ci-failed"), 1);
    }
}
