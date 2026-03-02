use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use ennio_core::id::{ProjectId, SessionId};
use ennio_core::lifecycle::SpawnRequest;
use ennio_core::session::Session;

use crate::response::{ApiError, ApiResponse};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SpawnBody {
    pub project_id: String,
    pub issue_id: Option<String>,
    pub prompt: Option<String>,
    pub branch: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendBody {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub activity: Option<String>,
    pub branch: Option<String>,
    pub pr_url: Option<String>,
    pub agent_name: Option<String>,
}

fn session_to_summary(s: &Session) -> SessionSummary {
    SessionSummary {
        id: s.id.to_string(),
        project_id: s.project_id.to_string(),
        status: s.status.to_string(),
        activity: s.activity.map(|a| a.to_string()),
        branch: s.branch.clone(), // clone: extracting from borrowed session into owned response
        pr_url: s.pr_url.clone(), // clone: extracting from borrowed session into owned response
        agent_name: s.agent_name.clone(), // clone: extracting from borrowed session into owned response
    }
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    project_id: Option<Path<String>>,
) -> Result<ApiResponse<Vec<SessionSummary>>, ApiError> {
    let pid = match &project_id {
        Some(Path(id)) => Some(ProjectId::new(id)?),
        None => None,
    };

    let sessions = state
        .session_manager
        .list(pid.as_ref())
        .await
        .map_err(ApiError::from)?;

    let summaries: Vec<SessionSummary> = sessions.iter().map(session_to_summary).collect();

    Ok(ApiResponse { data: summaries })
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<ApiResponse<SessionSummary>, ApiError> {
    let session_id = SessionId::new(id)?;
    let session = state
        .session_manager
        .get(&session_id)
        .await
        .map_err(ApiError::from)?;

    Ok(ApiResponse {
        data: session_to_summary(&session),
    })
}

pub async fn spawn_session(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SpawnBody>,
) -> Result<ApiResponse<SessionSummary>, ApiError> {
    let project_id = ProjectId::new(body.project_id)?;
    let request = SpawnRequest {
        project_id: &project_id,
        issue_id: body.issue_id.as_deref(),
        prompt: body.prompt.as_deref(),
        branch: body.branch.as_deref(),
        role: body.role.as_deref(),
    };

    let session = state
        .session_manager
        .spawn(&request)
        .await
        .map_err(ApiError::from)?;

    Ok(ApiResponse {
        data: session_to_summary(&session),
    })
}

pub async fn kill_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<ApiResponse<&'static str>, ApiError> {
    let session_id = SessionId::new(id)?;
    state
        .session_manager
        .kill(&session_id)
        .await
        .map_err(ApiError::from)?;

    Ok(ApiResponse { data: "killed" })
}

pub async fn send_to_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<SendBody>,
) -> Result<ApiResponse<&'static str>, ApiError> {
    let session_id = SessionId::new(id)?;
    state
        .session_manager
        .send(&session_id, &body.message)
        .await
        .map_err(ApiError::from)?;

    Ok(ApiResponse { data: "sent" })
}

pub async fn health() -> ApiResponse<&'static str> {
    ApiResponse { data: "ok" }
}
