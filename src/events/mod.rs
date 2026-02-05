//! Event system for publishing structured domain events.
//!
//! Events can be published to Kafka (with the `kafka` feature) or logged.

mod publisher;
mod types;

pub use publisher::{EventPublisher, LogEventPublisher};
pub use types::*;

#[cfg(feature = "kafka")]
mod kafka;
#[cfg(feature = "kafka")]
pub use kafka::KafkaEventPublisher;
