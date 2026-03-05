use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use tracing::{error, warn};

use ennio_core::event::{EventPriority, EventType, OrchestratorEvent};
use ennio_core::id::{EventId, ProjectId, SessionId};
use ennio_db::repo::events;
use ennio_nats::EventPublisher;

use crate::event_bus::EventBus;

pub(crate) struct EventContext<'a> {
    pub event_bus: &'a EventBus,
    pub pool: &'a SqlitePool,
    pub publisher: &'a Arc<EventPublisher>,
}

pub(crate) fn fire_event(
    ctx: &EventContext<'_>,
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

    ctx.event_bus.publish(&event);

    // clone: Arc reference count increment for async task ownership
    let publisher = Arc::clone(ctx.publisher);
    // clone: SqlitePool uses Arc internally, this is a cheap reference count increment
    let pool = ctx.pool.clone();
    tokio::spawn(async move {
        persist_event(&pool, &publisher, event).await;
    });
}

async fn persist_event(
    pool: &SqlitePool,
    publisher: &Arc<EventPublisher>,
    event: OrchestratorEvent,
) {
    if let Err(e) = publisher.publish_event(&event).await {
        warn!("failed to publish event to NATS: {e}");
    }
    if let Err(e) = events::insert(pool, &event).await {
        error!("failed to persist event: {e}");
    }
}
