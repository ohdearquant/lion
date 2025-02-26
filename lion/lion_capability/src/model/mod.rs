//! Capability models.
//! 
//! This module defines the core capability types and traits.

pub mod capability;
pub mod file;
pub mod network;
pub mod memory;
pub mod plugin_call;
pub mod message;
pub mod composite;

pub use capability::Capability;
pub use file::FileCapability;
pub use network::NetworkCapability;
pub use memory::MemoryCapability;
pub use plugin_call::PluginCallCapability;
pub use message::MessageCapability;
pub use composite::CompositeCapability;