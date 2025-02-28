//! Event-driven workflow components

pub mod broker;
pub mod retry;
pub mod store;
pub mod subscription;
pub mod types;

// Re-exports
pub use broker::EventBroker;
pub use retry::RetryManager;
pub use store::{EventStore, InMemoryEventStore};
pub use subscription::{EventSubscription, SerializableSubscription};
pub use types::{DeliverySemantic, Event, EventAck, EventError, EventPriority, EventStatus};

// Re-export the config to avoid the duplicate export warning
pub use types::EventBrokerConfig;

#[cfg(test)]
mod tests {
    // Import necessary items for testing
    // Add test modules here as needed
}
