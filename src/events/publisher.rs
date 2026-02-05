//! Event publisher trait and log-based implementation.

use super::Event;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for publishing events to a backend.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish an event to the given topic.
    async fn publish(&self, topic: &str, event: Event) -> Result<()>;

    /// Publish an event, automatically routing to the correct topic.
    async fn publish_auto(&self, event: Event) -> Result<()> {
        let topic = event.topic().to_string();
        self.publish(&topic, event).await
    }
}

/// Log-based event publisher (default when Kafka is not available).
pub struct LogEventPublisher;

#[async_trait]
impl EventPublisher for LogEventPublisher {
    async fn publish(&self, topic: &str, event: Event) -> Result<()> {
        tracing::info!(
            event_id = %event.id,
            event_type = %event.event_type,
            topic = %topic,
            user_id = ?event.metadata.user_id,
            "Event published (log)"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_publisher_succeeds() {
        let publisher = LogEventPublisher;
        let event = Event::new("test.event", serde_json::json!({"key": "value"}));
        let result = publisher.publish("test-topic", event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_log_publisher_auto_topic() {
        let publisher = LogEventPublisher;
        let event = Event::new("download.started", serde_json::json!({}));
        let result = publisher.publish_auto(event).await;
        assert!(result.is_ok());
    }
}
