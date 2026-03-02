use chrono::{DateTime, Utc};
use ennio_core::event::{EventPriority, EventType, OrchestratorEvent};
use ennio_core::id::{EventId, ProjectId, SessionId};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use crate::error::DbError;

fn map_event_row(row: &PgRow) -> Result<OrchestratorEvent, DbError> {
    let id_uuid: uuid::Uuid = row.try_get("id")?;
    let event_type_str: String = row.try_get("event_type")?;
    let priority_str: String = row.try_get("priority")?;
    let session_id_str: String = row.try_get("session_id")?;
    let project_id_str: String = row.try_get("project_id")?;
    let timestamp: DateTime<Utc> = row.try_get("timestamp")?;
    let message: String = row.try_get("message")?;
    let data: serde_json::Value = row.try_get("data")?;

    let id = EventId::new(id_uuid.to_string()).map_err(|e| DbError::RowMapping(e.to_string()))?;

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

pub async fn insert(pool: &PgPool, event: &OrchestratorEvent) -> Result<(), DbError> {
    let id_uuid: uuid::Uuid = event
        .id
        .as_str()
        .parse()
        .map_err(|e: uuid::Error| DbError::RowMapping(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO events (id, event_type, priority, session_id, project_id, timestamp, message, data)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(id_uuid)
    .bind(event.event_type.to_string())
    .bind(event.priority.to_string())
    .bind(event.session_id.as_str())
    .bind(event.project_id.as_str())
    .bind(event.timestamp)
    .bind(event.message.as_str())
    .bind(&event.data)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_by_session(
    pool: &PgPool,
    session_id: &SessionId,
) -> Result<Vec<OrchestratorEvent>, DbError> {
    let rows = sqlx::query("SELECT * FROM events WHERE session_id = $1 ORDER BY timestamp DESC")
        .bind(session_id.as_str())
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_event_row).collect()
}

pub async fn list_by_type(
    pool: &PgPool,
    event_type: EventType,
) -> Result<Vec<OrchestratorEvent>, DbError> {
    let rows = sqlx::query("SELECT * FROM events WHERE event_type = $1 ORDER BY timestamp DESC")
        .bind(event_type.to_string())
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_event_row).collect()
}

pub async fn list_recent(pool: &PgPool, limit: i64) -> Result<Vec<OrchestratorEvent>, DbError> {
    let rows = sqlx::query("SELECT * FROM events ORDER BY timestamp DESC LIMIT $1")
        .bind(limit)
        .fetch_all(pool)
        .await?;

    rows.iter().map(map_event_row).collect()
}
