//! Capability management for the Lion WebAssembly Plugin System.
//!
//! This crate provides the implementation of capability-based security
//! for the Lion WebAssembly Plugin System.

pub mod error;
pub mod manager;
pub mod policy;

// Re-exports for convenience
pub use error::CapabilityManagerError;
pub use manager::{CapabilityManagerImpl, CapabilityManagerImplConfig};
pub use policy::{CapabilityPolicy, DefaultCapabilityPolicy};