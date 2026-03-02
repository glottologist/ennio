use chrono::{DateTime, Utc};
use ennio_core::id::{ProjectId, SessionId};
use ennio_core::session::{ActivityState, Session, SessionStatus};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use crate::error::DbError;

fn map_session_row(row: &PgRow) -> Result<Session, DbError> {
    let id_str: String = row.try_get("id")?;
    let project_id_str: String = row.try_get("project_id")?;
    let status_str: String = row.try_get("status")?;
    let activity_str: Option<String> = row.try_get("activity")?;
    let branch: Option<String> = row.try_get("branch")?;
    let issue_id: Option<String> = row.try_get("issue_id")?;
    let workspace_path_str: Option<String> = row.try_get("workspace_path")?;
    let runtime_handle_json: Option<serde_json::Value> = row.try_get("runtime_handle")?;
    let agent_info_json: Option<serde_json::Value> = row.try_get("agent_info")?;
    let agent_name: Option<String> = row.try_get("agent_name")?;
    let pr_url: Option<String> = row.try_get("pr_url")?;
    let pr_number: Option<i32> = row.try_get("pr_number")?;
    let tmux_name: Option<String> = row.try_get("tmux_name")?;
    let config_hash: String = row.try_get("config_hash")?;
    let role: Option<String> = row.try_get("role")?;
    let metadata_json: serde_json::Value = row.try_get("metadata")?;
    let created_at: DateTime<Utc> = row.try_get("created_at")?;
    let last_activity_at: DateTime<Utc> = row.try_get("last_activity_at")?;
    let restored_at: Option<DateTime<Utc>> = row.try_get("restored_at")?;
    let archived_at: Option<DateTime<Utc>> = row.try_get("archived_at")?;

    let id = SessionId::new(id_str).map_err(|e| DbError::RowMapping(e.to_string()))?;
    let project_id =
        ProjectId::new(project_id_str).map_err(|e| DbError::RowMapping(e.to_string()))?;

    let status: SessionStatus = status_str
        .parse()
        .map_err(|e| DbError::RowMapping(format!("invalid session status: {e}")))?;

    let activity: Option<ActivityState> = activity_str
        .map(|s| {
            s.parse()
                .map_err(|e| DbError::RowMapping(format!("invalid activity state: {e}")))
        })
        .transpose()?;

    let workspace_path = workspace_path_str.map(std::path::PathBuf::from);

    let runtime_handle = runtime_handle_json
        .map(|v| serde_json::from_value(v).map_err(DbError::Json))
        .transpose()?;

    let agent_info = agent_info_json
        .map(|v| serde_json::from_value(v).map_err(DbError::Json))
        .transpose()?;

    let metadata = serde_json::from_value(metadata_json).map_err(DbError::Json)?;

    Ok(Session {
        id,
        project_id,
        status,
        activity,
        branch,
        issue_id,
        workspace_path,
        runtime_handle,
        agent_info,
        agent_name,
        pr_url,
        pr_number,
        tmux_name,
        config_hash,
        role,
        metadata,
        created_at,
        last_activity_at,
        restored_at,
        archived_at,
    })
}

pub async fn insert(pool: &PgPool, session: &Session) -> Result<(), DbError> {
    let workspace_path_str = session.workspace_path.as_ref().map(|p| p.to_string_lossy());
    let workspace_path_ref = workspace_path_str.as_deref();
    let runtime_handle_json = session
        .runtime_handle
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let agent_info_json = session
        .agent_info
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let metadata_json = serde_json::to_value(&session.metadata)?;

    sqlx::query(
        r#"
        INSERT INTO sessions (
            id, project_id, status, activity, branch, issue_id,
            workspace_path, runtime_handle, agent_info, agent_name,
            pr_url, pr_number, tmux_name, config_hash, role, metadata,
            created_at, last_activity_at, restored_at, archived_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10,
            $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20
        )
        "#,
    )
    .bind(session.id.as_str())
    .bind(session.project_id.as_str())
    .bind(session.status.to_string())
    .bind(session.activity.map(|a| a.to_string()))
    .bind(session.branch.as_deref())
    .bind(session.issue_id.as_deref())
    .bind(workspace_path_ref)
    .bind(&runtime_handle_json)
    .bind(&agent_info_json)
    .bind(session.agent_name.as_deref())
    .bind(session.pr_url.as_deref())
    .bind(session.pr_number)
    .bind(session.tmux_name.as_deref())
    .bind(session.config_hash.as_str())
    .bind(session.role.as_deref())
    .bind(&metadata_json)
    .bind(session.created_at)
    .bind(session.last_activity_at)
    .bind(session.restored_at)
    .bind(session.archived_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get(pool: &PgPool, id: &SessionId) -> Result<Option<Session>, DbError> {
    let row = sqlx::query("SELECT * FROM sessions WHERE id = $1")
        .bind(id.as_str())
        .fetch_optional(pool)
        .await?;

    row.as_ref().map(map_session_row).transpose()
}

pub async fn list(pool: &PgPool, project_id: Option<&ProjectId>) -> Result<Vec<Session>, DbError> {
    let rows = match project_id {
        Some(pid) => {
            sqlx::query("SELECT * FROM sessions WHERE project_id = $1 ORDER BY created_at DESC")
                .bind(pid.as_str())
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query("SELECT * FROM sessions ORDER BY created_at DESC")
                .fetch_all(pool)
                .await?
        }
    };

    rows.iter().map(map_session_row).collect()
}

pub async fn update_status(
    pool: &PgPool,
    id: &SessionId,
    status: SessionStatus,
) -> Result<(), DbError> {
    sqlx::query("UPDATE sessions SET status = $1, last_activity_at = NOW() WHERE id = $2")
        .bind(status.to_string())
        .bind(id.as_str())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_activity(
    pool: &PgPool,
    id: &SessionId,
    activity: Option<ActivityState>,
) -> Result<(), DbError> {
    sqlx::query("UPDATE sessions SET activity = $1, last_activity_at = NOW() WHERE id = $2")
        .bind(activity.map(|a| a.to_string()))
        .bind(id.as_str())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_pr(
    pool: &PgPool,
    id: &SessionId,
    pr_url: &str,
    pr_number: i32,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE sessions SET pr_url = $1, pr_number = $2, last_activity_at = NOW() WHERE id = $3",
    )
    .bind(pr_url)
    .bind(pr_number)
    .bind(id.as_str())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_metadata(
    pool: &PgPool,
    id: &SessionId,
    metadata: &serde_json::Value,
) -> Result<(), DbError> {
    sqlx::query("UPDATE sessions SET metadata = $1, last_activity_at = NOW() WHERE id = $2")
        .bind(metadata)
        .bind(id.as_str())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete(pool: &PgPool, id: &SessionId) -> Result<(), DbError> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(id.as_str())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_by_status(pool: &PgPool, status: SessionStatus) -> Result<Vec<Session>, DbError> {
    let rows = sqlx::query("SELECT * FROM sessions WHERE status = $1 ORDER BY created_at DESC")
        .bind(status.to_string())
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_session_row).collect()
}

pub async fn archive(pool: &PgPool, id: &SessionId) -> Result<(), DbError> {
    sqlx::query("UPDATE sessions SET archived_at = NOW(), last_activity_at = NOW() WHERE id = $1")
        .bind(id.as_str())
        .execute(pool)
        .await?;

    Ok(())
}
