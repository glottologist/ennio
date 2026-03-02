use bytes::Bytes;
use futures::StreamExt;
use serde::Serialize;

use crate::error::NatsError;

#[derive(Clone)]
pub struct NatsClient {
    inner: async_nats::Client,
}

impl NatsClient {
    pub async fn connect(url: &str) -> Result<Self, NatsError> {
        let client = async_nats::connect(url).await?;
        Ok(Self { inner: client })
    }

    pub async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), NatsError> {
        self.inner
            .publish(topic.to_string(), Bytes::copy_from_slice(payload))
            .await?;
        Ok(())
    }

    pub async fn publish_json<T: Serialize>(&self, topic: &str, data: &T) -> Result<(), NatsError> {
        let payload = serde_json::to_vec(data)?;
        self.publish(topic, &payload).await
    }

    pub async fn subscribe(&self, topic: &str) -> Result<NatsSubscription, NatsError> {
        let subscriber = self.inner.subscribe(topic.to_string()).await?;
        Ok(NatsSubscription { inner: subscriber })
    }
}

pub struct NatsSubscription {
    inner: async_nats::Subscriber,
}

impl NatsSubscription {
    pub async fn next(&mut self) -> Option<async_nats::Message> {
        self.inner.next().await
    }
}
