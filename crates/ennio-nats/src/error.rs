#[derive(Debug, thiserror::Error)]
pub enum NatsError {
    #[error("NATS connection error: {0}")]
    Connection(#[from] async_nats::ConnectError),

    #[error("NATS publish error: {0}")]
    Publish(#[from] async_nats::PublishError),

    #[error("NATS subscribe error: {0}")]
    Subscribe(#[from] async_nats::SubscribeError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid NATS topic: {0}")]
    InvalidTopic(String),

    #[error("NATS is not configured")]
    NotConfigured,
}
