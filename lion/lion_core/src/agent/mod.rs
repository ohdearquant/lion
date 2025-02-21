//! # Agent System
//!
//! This module defines the core agent abstractions and implementations for the Lion framework.
//! It provides the foundational components needed for creating and managing AI agents within
//! the system.
//!
//! ## Components
//!
//! - `protocol`: Defines the `AgentProtocol` trait that all agents must implement, establishing
//!   the contract for agent behavior and communication.
//!
//! - `events`: Contains event types and handlers specific to agent lifecycle and communication.
//!   These events are used for internal agent state management and external notifications.
//!
//! - `mock`: Provides a `MockStreamingAgent` implementation for testing and demonstration
//!   purposes. This implementation helps validate agent behavior and serves as a reference
//!   for creating new agent types.
//!
//! ## Usage
//!
//! Agents in the Lion framework follow a protocol-based design where each agent implements
//! the `AgentProtocol` trait. This ensures consistent behavior across different agent
//! implementations while allowing for specialized functionality.
//!
//! ```rust,no_run
//! use lion_core::agent::AgentProtocol;
//! // Example agent implementation would implement AgentProtocol
//! ```

mod events;
#[cfg(test)]
mod mock;
mod protocol;

pub use events::AgentEvent;
#[cfg(test)]
pub use mock::MockStreamingAgent;
pub use protocol::AgentProtocol;
