use chrono::{DateTime, Utc};
use ennio_core::event::{EventPriority, EventType, OrchestratorEvent};
use ennio_core::session::{ActivityState, Session, SessionStatus};

#[derive(Debug, Clone)]
pub struct SessionView {
    pub id: String,
    pub project_id: String,
    pub status: SessionStatus,
    pub activity: Option<ActivityState>,
    pub branch: Option<String>,
    pub agent_name: Option<String>,
    pub pr_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

impl SessionView {
    pub fn from_session(session: &Session) -> Self {
        Self {
            id: session.id.to_string(),
            project_id: session.project_id.to_string(),
            status: session.status,
            activity: session.activity,
            branch: session.branch.as_deref().map(String::from),
            agent_name: session.agent_name.as_deref().map(String::from),
            pr_url: session.pr_url.as_deref().map(String::from),
            created_at: session.created_at,
            last_activity_at: session.last_activity_at,
        }
    }

    pub fn status_label(&self) -> String {
        self.status.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct EventView {
    pub event_type: EventType,
    pub priority: EventPriority,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

impl EventView {
    pub fn from_event(event: &OrchestratorEvent) -> Self {
        Self {
            event_type: event.event_type,
            priority: event.priority,
            session_id: event.session_id.to_string(),
            timestamp: event.timestamp,
            message: event.message.as_str().into(),
        }
    }
}

pub struct App {
    pub sessions: Vec<SessionView>,
    pub selected_index: usize,
    pub show_detail: bool,
    pub events: Vec<EventView>,
    pub should_quit: bool,
}

impl App {
    pub fn new(sessions: Vec<SessionView>, events: Vec<EventView>) -> Self {
        Self {
            sessions,
            selected_index: 0,
            show_detail: false,
            events,
            should_quit: false,
        }
    }

    pub fn next(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.sessions.len();
    }

    pub fn previous(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        self.selected_index = self
            .selected_index
            .checked_sub(1)
            .unwrap_or(self.sessions.len() - 1);
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn selected_session(&self) -> Option<&SessionView> {
        self.sessions.get(self.selected_index)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn make_session_view(id: &str, status: SessionStatus) -> SessionView {
        SessionView {
            id: id.into(),
            project_id: "proj-1".into(),
            status,
            activity: None,
            branch: None,
            agent_name: None,
            pr_url: None,
            created_at: Utc::now(),
            last_activity_at: Utc::now(),
        }
    }

    fn make_app(count: usize) -> App {
        let sessions: Vec<SessionView> = (0..count)
            .map(|i| make_session_view(&format!("s-{i}"), SessionStatus::Working))
            .collect();
        App::new(sessions, vec![])
    }

    #[test]
    fn new_app_defaults() {
        let app = make_app(3);
        assert_eq!(app.selected_index, 0);
        assert!(!app.show_detail);
        assert!(!app.should_quit);
        assert_eq!(app.sessions.len(), 3);
    }

    #[rstest]
    #[case(0, 1)]
    #[case(1, 2)]
    #[case(2, 0)]
    fn next_wraps(#[case] start: usize, #[case] expected: usize) {
        let mut app = make_app(3);
        app.selected_index = start;
        app.next();
        assert_eq!(app.selected_index, expected);
    }

    #[rstest]
    #[case(0, 2)]
    #[case(1, 0)]
    #[case(2, 1)]
    fn previous_wraps(#[case] start: usize, #[case] expected: usize) {
        let mut app = make_app(3);
        app.selected_index = start;
        app.previous();
        assert_eq!(app.selected_index, expected);
    }

    #[test]
    fn next_empty_sessions_noop() {
        let mut app = make_app(0);
        app.next();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn previous_empty_sessions_noop() {
        let mut app = make_app(0);
        app.previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn toggle_detail_flips() {
        let mut app = make_app(1);
        assert!(!app.show_detail);
        app.toggle_detail();
        assert!(app.show_detail);
        app.toggle_detail();
        assert!(!app.show_detail);
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = make_app(0);
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn selected_session_returns_correct() {
        let app = make_app(3);
        let session = app.selected_session().unwrap();
        assert_eq!(session.id, "s-0");
    }

    #[test]
    fn selected_session_empty_returns_none() {
        let app = make_app(0);
        assert!(app.selected_session().is_none());
    }

    #[rstest]
    #[case(SessionStatus::Spawning, "spawning")]
    #[case(SessionStatus::Working, "working")]
    #[case(SessionStatus::PrOpen, "pr_open")]
    #[case(SessionStatus::PrDraft, "pr_draft")]
    #[case(SessionStatus::CiPassing, "ci_passing")]
    #[case(SessionStatus::CiFailed, "ci_failed")]
    #[case(SessionStatus::CiFixSent, "ci_fix_sent")]
    #[case(SessionStatus::CiFixFailed, "ci_fix_failed")]
    #[case(SessionStatus::ReviewPending, "review_pending")]
    #[case(SessionStatus::ChangesRequested, "changes_requested")]
    #[case(SessionStatus::Approved, "approved")]
    #[case(SessionStatus::MergeConflicts, "merge_conflicts")]
    #[case(SessionStatus::Merged, "merged")]
    #[case(SessionStatus::Done, "done")]
    #[case(SessionStatus::Exited, "exited")]
    #[case(SessionStatus::Killed, "killed")]
    fn status_label_correct(#[case] status: SessionStatus, #[case] expected: &str) {
        let view = make_session_view("test", status);
        assert_eq!(view.status_label(), expected);
    }

    #[rstest]
    #[case(SessionStatus::CiFailed)]
    #[case(SessionStatus::CiFixFailed)]
    #[case(SessionStatus::ChangesRequested)]
    #[case(SessionStatus::MergeConflicts)]
    fn needs_attention_returns_true(#[case] status: SessionStatus) {
        assert!(status.needs_attention());
        assert!(status.attention_reason().is_some());
    }

    #[rstest]
    #[case(SessionStatus::Spawning)]
    #[case(SessionStatus::Working)]
    #[case(SessionStatus::CiPassing)]
    #[case(SessionStatus::Approved)]
    #[case(SessionStatus::Merged)]
    #[case(SessionStatus::Done)]
    fn no_attention_returns_false(#[case] status: SessionStatus) {
        assert!(!status.needs_attention());
        assert!(status.attention_reason().is_none());
    }
}
