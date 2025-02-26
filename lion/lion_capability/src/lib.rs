//! # Lion Capability
//! 
//! `lion_capability` provides a capability-based security system for the
//! Lion microkernel. Capabilities are unforgeable tokens of authority that
//! grant specific permissions to access resources.
//! 
//! Key concepts:
//! 
//! 1. **Capability**: An unforgeable token of authority that grants specific permissions.
//! 
//! 2. **Partial Revocation**: The ability to revoke only specific parts of a capability.
//! 
//! 3. **Capability Composition**: The ability to combine capabilities.
//! 
//! 4. **Attenuation**: The principle that derived capabilities can only restrict permissions,
//!    never add new ones.

pub mod model;
pub mod store;
pub mod check;
pub mod attenuation;

// Re-export key types and traits for convenience
pub use model::{
    Capability, FileCapability, NetworkCapability, MemoryCapability, 
    PluginCallCapability, MessageCapability, CustomCapability
};
pub use store::{CapabilityStore, InMemoryCapabilityStore};
pub use check::CapabilityChecker;
pub use attenuation::{FilterCapability, ProxyCapability};