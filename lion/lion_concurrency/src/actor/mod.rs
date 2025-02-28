//! Actor-based concurrency system with message passing and supervision.
//!
//! This module provides the core actor model implementation for Lion, including:
//!
//! - Message passing via typed mailboxes
//! - Actor supervision and failure recovery
//! - Actor system for managing actor lifecycles

pub mod mailbox;
pub mod supervisor;
pub mod system;

// Re-export key types from mailbox
pub use mailbox::{Mailbox, MailboxError, Message};

// Re-export key types from supervisor
pub use supervisor::{
    BasicSupervisor, SupervisionError, SupervisionStrategy, Supervisor, SupervisorConfig,
};

// Re-export key types from system
pub use system::{ActorId, ActorStatus, ActorSystem, ActorSystemConfig};

#[cfg(test)]
mod tests {
    // Integration tests for the actor module can be added here
}
