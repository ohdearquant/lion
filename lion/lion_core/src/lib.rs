//! Core definitions for the Lion WebAssembly Plugin System.
//!
//! This crate defines the key traits and data structures that form the foundation
//! of the Lion WebAssembly Plugin System. It includes definitions for plugins,
//! capabilities, messaging, and resource monitoring.

pub mod capability;
pub mod error;
pub mod message;
pub mod plugin;
pub mod resource;

// Re-export key items for convenience
pub use capability::{Capability, CapabilityId, CapabilityManager, CoreCapability};
pub use error::{CapabilityError, MessageError, PluginError, ResourceError};
pub use message::{Message, MessageBus, TopicId};
pub use plugin::{Plugin, PluginId, PluginManager, PluginManifest, PluginSource, PluginState};
pub use resource::ResourceMonitor;

// Create a type alias for Result with our error types
pub type Result<T, E = error::Error> = std::result::Result<T, E>;