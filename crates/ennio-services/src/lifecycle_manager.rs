use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use ennio_core::config::OrchestratorConfig;
use ennio_core::error::EnnioError;
use ennio_core::event::{EventPriority, EventType};
use ennio_core::id::{ProjectId, SessionId};
use ennio_core::lifecycle::{LifecycleManager, SessionManager, SessionState};
use ennio_core::reaction::ReactionConfig;
use ennio_core::scm::{CIStatus, PRState, ReviewDecision};
use ennio_core::session::{Session, SessionStatus};
use ennio_db::repo::sessions;
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
            .get(&(session_id.clone(), reaction_key.to_owned())) // clone: HashMap lookup requires owned tuple key
            .copied()
            .unwrap_or(0)
    }

    fn increment_attempts(&mut self, session_id: &SessionId, reaction_key: &str) {
        let key = (session_id.clone(), reaction_key.to_owned()); // clone: HashMap key needs owned value
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
    pool: SqlitePool,
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
        pool: SqlitePool,
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
        let project = self.find_project(session)?;

        if !self.is_runtime_alive(session, project).await? {
            return self.transition_to_exited(session).await;
        }

        if !self.is_agent_alive(session, project).await? {
            return self.transition_to_exited(session).await;
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

    fn find_project<'a>(
        &'a self,
        session: &Session,
    ) -> Result<&'a ennio_core::config::ProjectConfig, EnnioError> {
        self.config.find_project(&session.project_id)
    }

    async fn is_runtime_alive(
        &self,
        session: &Session,
        project: &ennio_core::config::ProjectConfig,
    ) -> Result<bool, EnnioError> {
        let is_node_strategy = project
            .ssh_config
            .as_ref()
            .is_some_and(|ssh| ssh.strategy == ennio_core::config::SshStrategyConfig::Node);

        if is_node_strategy {
            if session.runtime_handle.is_some() {
                self.emit_event(
                    EventType::NodeHealthCheck,
                    EventPriority::Info,
                    &session.id,
                    &session.project_id,
                    "node health check via session manager",
                );
                return Ok(true);
            }
            return Ok(false);
        }

        let Some(ref handle) = session.runtime_handle else {
            return Ok(false);
        };

        let runtime_name = project
            .runtime
            .as_deref()
            .unwrap_or(self.config.defaults.runtime.as_str());
        let runtime = self.registry.get_runtime(runtime_name)?;
        Ok(runtime.is_alive(handle).await.unwrap_or(false))
    }

    async fn is_agent_alive(
        &self,
        session: &Session,
        project: &ennio_core::config::ProjectConfig,
    ) -> Result<bool, EnnioError> {
        let Some(ref handle) = session.runtime_handle else {
            return Ok(true);
        };

        let agent_name = project
            .agent
            .as_deref()
            .unwrap_or(self.config.defaults.agent.as_str());
        let agent_plugin = self.registry.get_agent(agent_name)?;
        agent_plugin.is_process_running(handle).await
    }

    async fn transition_to_exited(&self, session: &Session) -> Result<SessionState, EnnioError> {
        self.transition_status(session, SessionStatus::Exited)
            .await?;
        Ok(SessionState {
            // clone: SessionId must be owned by SessionState
            session_id: session.id.clone(),
            status: SessionStatus::Exited,
            last_checked: Utc::now(),
        })
    }

    async fn resolve_pr_number(
        &self,
        session: &Session,
        project: &ennio_core::config::ProjectConfig,
    ) -> Option<i32> {
        if let Some(n) = session.pr_number {
            return Some(n);
        }

        let branch = session.branch.as_deref()?;
        let scm_config = project.scm_config.as_ref()?;
        let scm = self.registry.get_scm(&scm_config.plugin).ok()?;

        scm.detect_pr(&session.project_id, branch)
            .await
            .ok()
            .flatten()
            .map(|pr| pr.number)
    }

    async fn determine_status_from_external(
        &self,
        session: &Session,
        project: &ennio_core::config::ProjectConfig,
    ) -> SessionStatus {
        let pr_number = match self.resolve_pr_number(session, project).await {
            Some(n) => n,
            None => return session.status,
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

        determine_status(ci_status, review_decision, pr_state, session.status)
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
                self.execute_send_to_agent(session, reaction_key, reaction_config)
                    .await;
            }
            ennio_core::reaction::ReactionAction::Notify => {
                self.execute_notify(session, reaction_key, reaction_config);
            }
            ennio_core::reaction::ReactionAction::AutoMerge => {
                if !self.execute_auto_merge(session).await {
                    return;
                }
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

    async fn execute_send_to_agent(
        &self,
        session: &Session,
        reaction_key: &str,
        reaction_config: &ReactionConfig,
    ) {
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

    fn execute_notify(
        &self,
        session: &Session,
        reaction_key: &str,
        reaction_config: &ReactionConfig,
    ) {
        self.emit_event(
            EventType::ReactionTriggered,
            reaction_config.priority,
            &session.id,
            &session.project_id,
            &format!("notification reaction triggered: {reaction_key}"),
        );
    }

    async fn execute_auto_merge(&self, session: &Session) -> bool {
        let pr_number = match session.pr_number {
            Some(n) => n,
            None => {
                warn!(session_id = %session.id, "auto-merge skipped: no PR number");
                return false;
            }
        };

        let project = self.config.projects.iter().find(|p| {
            p.project_id
                .as_ref()
                .is_some_and(|id| id == &session.project_id)
        });

        let scm_config = match project.and_then(|p| p.scm_config.as_ref()) {
            Some(c) => c,
            None => {
                warn!(session_id = %session.id, "auto-merge skipped: no SCM config");
                return false;
            }
        };

        let scm = match self.registry.get_scm(&scm_config.plugin) {
            Ok(s) => s,
            Err(e) => {
                error!(
                    session_id = %session.id,
                    error = %e,
                    "auto-merge skipped: SCM plugin not found"
                );
                return false;
            }
        };

        match scm.merge_pr(&session.project_id, pr_number).await {
            Ok(()) => {
                info!(session_id = %session.id, pr_number, "auto-merge completed");
                true
            }
            Err(e) => {
                error!(
                    session_id = %session.id,
                    pr_number,
                    error = %e,
                    "auto-merge failed"
                );
                false
            }
        }
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
        let ctx = crate::events::EventContext {
            event_bus: &self.event_bus,
            pool: &self.pool,
            publisher: &self.publisher,
        };
        crate::events::fire_event(&ctx, event_type, priority, session_id, project_id, message);
    }
}

fn determine_status(
    ci_status: CIStatus,
    review_decision: ReviewDecision,
    pr_state: PRState,
    current_status: SessionStatus,
) -> SessionStatus {
    match (ci_status, review_decision) {
        (CIStatus::Failing, _) => {
            if current_status == SessionStatus::CiFixSent {
                SessionStatus::CiFixFailed
            } else {
                SessionStatus::CiFailed
            }
        }
        (CIStatus::Passing, ReviewDecision::Approved) => SessionStatus::Approved,
        (CIStatus::Passing, ReviewDecision::ChangesRequested) => SessionStatus::ChangesRequested,
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

    #[rstest]
    #[case(
        CIStatus::Failing,
        ReviewDecision::Pending,
        PRState::Open,
        SessionStatus::Working,
        SessionStatus::CiFailed
    )]
    #[case(
        CIStatus::Failing,
        ReviewDecision::Pending,
        PRState::Open,
        SessionStatus::CiFixSent,
        SessionStatus::CiFixFailed
    )]
    #[case(
        CIStatus::Passing,
        ReviewDecision::Approved,
        PRState::Open,
        SessionStatus::Working,
        SessionStatus::Approved
    )]
    #[case(
        CIStatus::Passing,
        ReviewDecision::ChangesRequested,
        PRState::Open,
        SessionStatus::Working,
        SessionStatus::ChangesRequested
    )]
    #[case(
        CIStatus::Passing,
        ReviewDecision::Pending,
        PRState::Open,
        SessionStatus::Working,
        SessionStatus::ReviewPending
    )]
    #[case(
        CIStatus::Pending,
        ReviewDecision::ChangesRequested,
        PRState::Open,
        SessionStatus::Working,
        SessionStatus::ChangesRequested
    )]
    #[case(
        CIStatus::Pending,
        ReviewDecision::Pending,
        PRState::Draft,
        SessionStatus::Working,
        SessionStatus::PrDraft
    )]
    #[case(
        CIStatus::Pending,
        ReviewDecision::Pending,
        PRState::Open,
        SessionStatus::Working,
        SessionStatus::PrOpen
    )]
    fn determine_status_from_ci_review(
        #[case] ci: CIStatus,
        #[case] review: ReviewDecision,
        #[case] pr_state: PRState,
        #[case] current: SessionStatus,
        #[case] expected: SessionStatus,
    ) {
        assert_eq!(determine_status(ci, review, pr_state, current), expected);
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
