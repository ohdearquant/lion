//! Agentic Core - Core primitives and logic for the lion project
//!
//! This library provides the foundational data structures and logic for
//! building an event-driven orchestration system.

pub mod element;
pub mod pile;
pub mod progression;
pub mod store;

// Re-export commonly used types
pub use element::ElementData;
pub use pile::Pile;
pub use progression::Progression;
pub use store::InMemoryStore;
