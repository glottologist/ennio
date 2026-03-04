use chrono::{DateTime, Utc};
use ennio_core::id::{ProjectId, SessionId};
use ennio_core::session::{ActivityState, Session, SessionStatus};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::error::DbError;

fn map_session_row(row: &SqliteRow) -> Result<Session, DbError> {
    let id_str: String = row.try_get("id")?;
    let project_id_str: String = row.try_get("project_id")?;
    let status_str: String = row.try_get("status")?;
    let activity_str: Option<String> = row.try_get("activity")?;
    let branch: Option<String> = row.try_get("branch")?;
    let issue_id: Option<String> = row.try_get("issue_id")?;
    let workspace_path_str: Option<String> = row.try_get("workspace_path")?;
    let runtime_handle_str: Option<String> = row.try_get("runtime_handle")?;
    let agent_info_str: Option<String> = row.try_get("agent_info")?;
    let agent_name: Option<String> = row.try_get("agent_name")?;
    let pr_url: Option<String> = row.try_get("pr_url")?;
    let pr_number: Option<i32> = row.try_get("pr_number")?;
    let tmux_name: Option<String> = row.try_get("tmux_name")?;
    let config_hash: String = row.try_get("config_hash")?;
    let role: Option<String> = row.try_get("role")?;
    let metadata_str: String = row.try_get("metadata")?;
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

    let runtime_handle = runtime_handle_str
        .map(|s| serde_json::from_str(&s).map_err(DbError::Json))
        .transpose()?;

    let agent_info = agent_info_str
        .map(|s| serde_json::from_str(&s).map_err(DbError::Json))
        .transpose()?;

    let metadata = serde_json::from_str(&metadata_str).map_err(DbError::Json)?;

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

pub async fn insert(pool: &SqlitePool, session: &Session) -> Result<(), DbError> {
    let workspace_path_str = session.workspace_path.as_ref().map(|p| p.to_string_lossy());
    let workspace_path_ref = workspace_path_str.as_deref();
    let runtime_handle_json = session
        .runtime_handle
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let agent_info_json = session
        .agent_info
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let metadata_json = serde_json::to_string(&session.metadata)?;

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

pub async fn get(pool: &SqlitePool, id: &SessionId) -> Result<Option<Session>, DbError> {
    let row = sqlx::query("SELECT * FROM sessions WHERE id = $1")
        .bind(id.as_str())
        .fetch_optional(pool)
        .await?;

    row.as_ref().map(map_session_row).transpose()
}

pub async fn list(
    pool: &SqlitePool,
    project_id: Option<&ProjectId>,
) -> Result<Vec<Session>, DbError> {
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
    pool: &SqlitePool,
    id: &SessionId,
    status: SessionStatus,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE sessions SET status = $1, last_activity_at = datetime('now') WHERE id = $2",
    )
    .bind(status.to_string())
    .bind(id.as_str())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_activity(
    pool: &SqlitePool,
    id: &SessionId,
    activity: Option<ActivityState>,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE sessions SET activity = $1, last_activity_at = datetime('now') WHERE id = $2",
    )
    .bind(activity.map(|a| a.to_string()))
    .bind(id.as_str())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_pr(
    pool: &SqlitePool,
    id: &SessionId,
    pr_url: &str,
    pr_number: i32,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE sessions SET pr_url = $1, pr_number = $2, last_activity_at = datetime('now') WHERE id = $3",
    )
    .bind(pr_url)
    .bind(pr_number)
    .bind(id.as_str())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_metadata(
    pool: &SqlitePool,
    id: &SessionId,
    metadata: &serde_json::Value,
) -> Result<(), DbError> {
    let metadata_str = serde_json::to_string(metadata)?;
    sqlx::query(
        "UPDATE sessions SET metadata = $1, last_activity_at = datetime('now') WHERE id = $2",
    )
    .bind(&metadata_str)
    .bind(id.as_str())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &SessionId) -> Result<(), DbError> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(id.as_str())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_by_status(
    pool: &SqlitePool,
    status: SessionStatus,
) -> Result<Vec<Session>, DbError> {
    let rows = sqlx::query("SELECT * FROM sessions WHERE status = $1 ORDER BY created_at DESC")
        .bind(status.to_string())
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_session_row).collect()
}

pub async fn archive(pool: &SqlitePool, id: &SessionId) -> Result<(), DbError> {
    sqlx::query("UPDATE sessions SET archived_at = datetime('now'), last_activity_at = datetime('now') WHERE id = $1")
        .bind(id.as_str())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use ennio_core::id::{ProjectId, SessionId};
    use ennio_core::session::{Session, SessionStatus};
    use proptest::prelude::*;

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = crate::pool::connect("sqlite::memory:").await.unwrap();
        crate::migrations::run_all(&pool).await.unwrap();
        pool
    }

    fn make_session(id_str: &str, project_str: &str) -> Session {
        Session {
            id: SessionId::new(id_str).unwrap(),
            project_id: ProjectId::new(project_str).unwrap(),
            status: SessionStatus::Working,
            activity: None,
            branch: None,
            issue_id: None,
            workspace_path: None,
            runtime_handle: None,
            agent_info: None,
            agent_name: None,
            pr_url: None,
            pr_number: None,
            tmux_name: None,
            config_hash: String::from("abc123"),
            role: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            last_activity_at: Utc::now(),
            restored_at: None,
            archived_at: None,
        }
    }

    proptest! {
        #[test]
        fn session_insert_get_roundtrip(
            id in "[a-zA-Z][a-zA-Z0-9_-]{0,31}",
            project in "[a-zA-Z][a-zA-Z0-9_-]{0,31}",
            config_hash in "[a-zA-Z0-9]{1,32}",
            branch in proptest::option::of("[a-zA-Z][a-zA-Z0-9_-]{0,15}"),
            agent_name in proptest::option::of("[a-zA-Z][a-zA-Z0-9_-]{0,15}"),
            role in proptest::option::of("[a-zA-Z][a-zA-Z0-9_-]{0,15}"),
        ) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let pool = test_pool().await;

                let session = Session {
                    id: SessionId::new(&id).unwrap(),
                    project_id: ProjectId::new(&project).unwrap(),
                    status: SessionStatus::Working,
                    activity: None,
                    branch,
                    issue_id: None,
                    workspace_path: None,
                    runtime_handle: None,
                    agent_info: None,
                    agent_name,
                    pr_url: None,
                    pr_number: None,
                    tmux_name: None,
                    config_hash,
                    role,
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                    last_activity_at: Utc::now(),
                    restored_at: None,
                    archived_at: None,
                };

                super::insert(&pool, &session).await.unwrap();
                let fetched = super::get(&pool, &session.id).await.unwrap().unwrap();

                prop_assert_eq!(fetched.id.as_str(), session.id.as_str());
                prop_assert_eq!(fetched.project_id.as_str(), session.project_id.as_str());
                prop_assert_eq!(fetched.status, session.status);
                prop_assert_eq!(fetched.branch, session.branch);
                prop_assert_eq!(fetched.agent_name, session.agent_name);
                prop_assert_eq!(fetched.config_hash, session.config_hash);
                prop_assert_eq!(fetched.role, session.role);
                Ok(())
            })?;
        }
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown_id() {
        let pool = test_pool().await;
        let unknown = SessionId::new("nonexistent-id").unwrap();
        let result = super::get(&pool, &unknown).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_filters_by_project() {
        let pool = test_pool().await;

        let s1 = make_session("sess-alpha", "project-one");
        let s2 = make_session("sess-beta", "project-one");
        let s3 = make_session("sess-gamma", "project-two");

        super::insert(&pool, &s1).await.unwrap();
        super::insert(&pool, &s2).await.unwrap();
        super::insert(&pool, &s3).await.unwrap();

        let proj_one = ProjectId::new("project-one").unwrap();
        let proj_two = ProjectId::new("project-two").unwrap();

        let list_one = super::list(&pool, Some(&proj_one)).await.unwrap();
        assert_eq!(list_one.len(), 2);
        for s in &list_one {
            assert_eq!(s.project_id.as_str(), "project-one");
        }

        let list_two = super::list(&pool, Some(&proj_two)).await.unwrap();
        assert_eq!(list_two.len(), 1);
        assert_eq!(list_two[0].project_id.as_str(), "project-two");

        let list_all = super::list(&pool, None).await.unwrap();
        assert_eq!(list_all.len(), 3);
    }
}
