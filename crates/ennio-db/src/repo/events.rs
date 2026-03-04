use chrono::{DateTime, Utc};
use ennio_core::event::{EventPriority, EventType, OrchestratorEvent};
use ennio_core::id::{EventId, ProjectId, SessionId};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use crate::error::DbError;

fn map_event_row(row: &SqliteRow) -> Result<OrchestratorEvent, DbError> {
    let id_str: String = row.try_get("id")?;
    let event_type_str: String = row.try_get("event_type")?;
    let priority_str: String = row.try_get("priority")?;
    let session_id_str: String = row.try_get("session_id")?;
    let project_id_str: String = row.try_get("project_id")?;
    let timestamp: DateTime<Utc> = row.try_get("timestamp")?;
    let message: String = row.try_get("message")?;
    let data_str: String = row.try_get("data")?;

    let id = EventId::new(id_str).map_err(|e| DbError::RowMapping(e.to_string()))?;

    let event_type: EventType = event_type_str
        .parse()
        .map_err(|e| DbError::RowMapping(format!("invalid event type: {e}")))?;

    let priority: EventPriority = priority_str
        .parse()
        .map_err(|e| DbError::RowMapping(format!("invalid event priority: {e}")))?;

    let session_id =
        SessionId::new(session_id_str).map_err(|e| DbError::RowMapping(e.to_string()))?;

    let project_id =
        ProjectId::new(project_id_str).map_err(|e| DbError::RowMapping(e.to_string()))?;

    let data: serde_json::Value = serde_json::from_str(&data_str).map_err(DbError::Json)?;

    Ok(OrchestratorEvent {
        id,
        event_type,
        priority,
        session_id,
        project_id,
        timestamp,
        message,
        data,
    })
}

pub async fn insert(pool: &SqlitePool, event: &OrchestratorEvent) -> Result<(), DbError> {
    let data_str = serde_json::to_string(&event.data)?;

    sqlx::query(
        r#"
        INSERT INTO events (id, event_type, priority, session_id, project_id, timestamp, message, data)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(event.id.as_str())
    .bind(event.event_type.to_string())
    .bind(event.priority.to_string())
    .bind(event.session_id.as_str())
    .bind(event.project_id.as_str())
    .bind(event.timestamp)
    .bind(event.message.as_str())
    .bind(&data_str)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_by_session(
    pool: &SqlitePool,
    session_id: &SessionId,
) -> Result<Vec<OrchestratorEvent>, DbError> {
    let rows = sqlx::query("SELECT * FROM events WHERE session_id = $1 ORDER BY timestamp DESC")
        .bind(session_id.as_str())
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_event_row).collect()
}

pub async fn list_by_type(
    pool: &SqlitePool,
    event_type: EventType,
) -> Result<Vec<OrchestratorEvent>, DbError> {
    let rows = sqlx::query("SELECT * FROM events WHERE event_type = $1 ORDER BY timestamp DESC")
        .bind(event_type.to_string())
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_event_row).collect()
}

pub async fn list_recent(pool: &SqlitePool, limit: i64) -> Result<Vec<OrchestratorEvent>, DbError> {
    let rows = sqlx::query("SELECT * FROM events ORDER BY timestamp DESC LIMIT $1")
        .bind(limit)
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_event_row).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use ennio_core::event::{EventPriority, EventType, OrchestratorEvent};
    use ennio_core::id::{EventId, ProjectId, SessionId};
    use ennio_core::session::{Session, SessionStatus};
    use proptest::prelude::*;

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = crate::pool::connect("sqlite::memory:").await.unwrap();
        crate::migrations::run_all(&pool).await.unwrap();
        pool
    }

    fn make_parent_session(session_id: &SessionId, project_id: &ProjectId) -> Session {
        Session {
            id: session_id.clone(), // clone: required to construct owned Session from borrowed id
            project_id: project_id.clone(), // clone: required to construct owned Session from borrowed id
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
            config_hash: String::from("hash"),
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
        fn event_insert_get_roundtrip(
            message in "[a-zA-Z0-9 ]{1,64}",
        ) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let pool = test_pool().await;

                let session_id = SessionId::new("test-session").unwrap();
                let project_id = ProjectId::new("test-project").unwrap();

                let parent = make_parent_session(&session_id, &project_id);
                crate::repo::sessions::insert(&pool, &parent).await.unwrap();

                let event = OrchestratorEvent {
                    id: EventId::random(),
                    event_type: EventType::SessionWorking,
                    priority: EventPriority::Info,
                    session_id: session_id.clone(), // clone: need owned copy for event struct
                    project_id: project_id.clone(), // clone: need owned copy for event struct
                    timestamp: Utc::now(),
                    message,
                    data: serde_json::json!({"key": "value"}),
                };

                super::insert(&pool, &event).await.unwrap();
                let events = super::list_by_session(&pool, &session_id).await.unwrap();

                prop_assert_eq!(events.len(), 1);
                let fetched = &events[0];
                prop_assert_eq!(fetched.id.as_str(), event.id.as_str());
                prop_assert_eq!(fetched.event_type, event.event_type);
                prop_assert_eq!(fetched.priority, event.priority);
                prop_assert_eq!(fetched.session_id.as_str(), event.session_id.as_str());
                prop_assert_eq!(fetched.project_id.as_str(), event.project_id.as_str());
                prop_assert_eq!(&fetched.message, &event.message);
                prop_assert_eq!(&fetched.data, &event.data);
                Ok(())
            })?;
        }
    }

    #[tokio::test]
    async fn list_by_session_returns_only_matching_events() {
        let pool = test_pool().await;

        let sid_a = SessionId::new("session-a").unwrap();
        let sid_b = SessionId::new("session-b").unwrap();
        let pid = ProjectId::new("test-project").unwrap();

        let parent_a = make_parent_session(&sid_a, &pid);
        let parent_b = make_parent_session(&sid_b, &pid);
        crate::repo::sessions::insert(&pool, &parent_a)
            .await
            .unwrap();
        crate::repo::sessions::insert(&pool, &parent_b)
            .await
            .unwrap();

        let event_a = OrchestratorEvent {
            id: EventId::random(),
            event_type: EventType::SessionSpawned,
            priority: EventPriority::Info,
            session_id: sid_a.clone(), // clone: need owned copy for event struct
            project_id: pid.clone(),   // clone: need owned copy for event struct
            timestamp: Utc::now(),
            message: String::from("spawned a"),
            data: serde_json::Value::Null,
        };
        let event_b = OrchestratorEvent {
            id: EventId::random(),
            event_type: EventType::SessionWorking,
            priority: EventPriority::Action,
            session_id: sid_b.clone(), // clone: need owned copy for event struct
            project_id: pid.clone(),   // clone: need owned copy for event struct
            timestamp: Utc::now(),
            message: String::from("working b"),
            data: serde_json::Value::Null,
        };

        super::insert(&pool, &event_a).await.unwrap();
        super::insert(&pool, &event_b).await.unwrap();

        let events_a = super::list_by_session(&pool, &sid_a).await.unwrap();
        assert_eq!(events_a.len(), 1);
        assert_eq!(events_a[0].message, "spawned a");

        let events_b = super::list_by_session(&pool, &sid_b).await.unwrap();
        assert_eq!(events_b.len(), 1);
        assert_eq!(events_b[0].message, "working b");
    }
}
