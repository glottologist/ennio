use std::sync::Arc;

use sqlx::SqlitePool;
use tracing::{error, warn};

use ennio_core::event::OrchestratorEvent;
use ennio_db::repo::events;
use ennio_nats::EventPublisher;

pub(crate) async fn emit_event(
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
