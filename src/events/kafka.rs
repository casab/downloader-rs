//! Kafka-backed event publisher (requires `kafka` feature).

use super::{Event, EventsConfig};
use super::publisher::EventPublisher;
use anyhow::{Context, Result};
use async_trait::async_trait;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::time::Duration;

/// Kafka event publisher using rdkafka.
pub struct KafkaEventPublisher {
    producer: FutureProducer,
}

impl KafkaEventPublisher {
    /// Create a new Kafka event publisher.
    pub fn new(config: &EventsConfig) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.kafka.brokers.join(","))
            .set("client.id", &config.kafka.client_id)
            .set("acks", &config.kafka.producer.acks)
            .set("retries", config.kafka.producer.retries.to_string())
            .set("linger.ms", config.kafka.producer.linger_ms.to_string())
            .set("batch.size", config.kafka.producer.batch_size.to_string())
            .create()
            .context("Failed to create Kafka producer")?;

        Ok(Self { producer })
    }
}

#[async_trait]
impl EventPublisher for KafkaEventPublisher {
    #[tracing::instrument(skip(self, event), fields(event_type = %event.event_type))]
    async fn publish(&self, topic: &str, event: Event) -> Result<()> {
        let key = event.id.to_string();
        let payload = serde_json::to_string(&event)
            .context("Failed to serialize event")?;

        let record = FutureRecord::to(topic)
            .key(&key)
            .payload(&payload);

        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| anyhow::anyhow!("Kafka send error: {e}"))?;

        tracing::debug!(event_id = %event.id, "Event published to Kafka");
        Ok(())
    }
}
