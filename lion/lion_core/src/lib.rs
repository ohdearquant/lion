//! # Lion Core
//! 
//! Core traits and data structures for the Lion WebAssembly Plugin System.
//! 
//! This crate defines the fundamental interfaces and types that form the
//! foundation of the Lion architecture:
//! 
//! - The capability system for secure resource access
//! - Plugin interfaces and lifecycle management
//! - Message passing for inter-plugin communication
//! - Resource monitoring and limiting
//! - Isolation backend abstraction for plugin execution
//! 
//! The `lion_core` crate is deliberately minimal and focuses on defining
//! interfaces rather than implementations. Concrete implementations of these
//! traits are provided by other crates in the Lion ecosystem.

pub mod capability;
pub mod error;
pub mod isolation;
pub mod message;
pub mod plugin;
pub mod resource;

// Re-export key items for convenience
pub use capability::{Capability, CapabilityId, CapabilityManager, CoreCapability};
pub use error::{CapabilityError, Error, MessageError, PluginError, ResourceError};
pub use isolation::IsolationBackend;
pub use message::{Message, MessageBus, TopicId};
pub use plugin::{Plugin, PluginId, PluginManager, PluginManifest, PluginSource, PluginState};
pub use resource::{ResourceLimits, ResourceMonitor, ResourceUsage};

/// A type alias for Result with our error types
pub type Result<T, E = error::Error> = std::result::Result<T, E>;