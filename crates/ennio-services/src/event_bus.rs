use ennio_core::event::OrchestratorEvent;
use tokio::sync::broadcast;
use tracing::debug;

const DEFAULT_CAPACITY: usize = 1024;

pub struct EventBus {
    sender: broadcast::Sender<OrchestratorEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(DEFAULT_CAPACITY);
        Self { sender }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: &OrchestratorEvent) {
        // clone: broadcast::Sender::send requires ownership of the value
        match self.sender.send(event.clone()) {
            Ok(receiver_count) => {
                debug!(
                    event_type = %event.event_type,
                    session_id = %event.session_id,
                    receivers = receiver_count,
                    "event published"
                );
            }
            Err(_) => {
                debug!(
                    event_type = %event.event_type,
                    session_id = %event.session_id,
                    "event published with no active receivers"
                );
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OrchestratorEvent> {
        self.sender.subscribe()
    }

    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ennio_core::event::{EventPriority, EventType};
    use ennio_core::id::{EventId, ProjectId, SessionId};

    use super::*;

    fn make_test_event(event_type: EventType) -> OrchestratorEvent {
        OrchestratorEvent {
            id: EventId::random(),
            event_type,
            priority: EventPriority::Info,
            session_id: SessionId::new("test-session").unwrap(),
            project_id: ProjectId::new("test-project").unwrap(),
            timestamp: Utc::now(),
            message: "test event".to_owned(),
            data: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn publish_and_receive() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        let event = make_test_event(EventType::SessionSpawned);
        bus.publish(&event);

        let received = rx.recv().await.unwrap();
        assert_eq!(received.event_type, EventType::SessionSpawned);
        assert_eq!(received.session_id.as_str(), "test-session");
    }

    #[tokio::test]
    async fn multiple_subscribers_receive() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = make_test_event(EventType::PrCreated);
        bus.publish(&event);

        let r1 = rx1.recv().await.unwrap();
        let r2 = rx2.recv().await.unwrap();
        assert_eq!(r1.event_type, EventType::PrCreated);
        assert_eq!(r2.event_type, EventType::PrCreated);
    }

    #[test]
    fn publish_without_subscribers_does_not_panic() {
        let bus = EventBus::new();
        let event = make_test_event(EventType::SessionKilled);
        bus.publish(&event);
    }

    #[test]
    fn receiver_count_tracks_subscriptions() {
        let bus = EventBus::new();
        assert_eq!(bus.receiver_count(), 0);

        let _rx1 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);
    }

    #[test]
    fn with_capacity_creates_bus() {
        let bus = EventBus::with_capacity(64);
        assert_eq!(bus.receiver_count(), 0);
    }

    #[test]
    fn default_creates_bus() {
        let bus = EventBus::default();
        assert_eq!(bus.receiver_count(), 0);
    }
}
