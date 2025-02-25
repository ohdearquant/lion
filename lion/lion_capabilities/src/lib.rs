//! # Lion Capabilities
//!
//! This crate implements the capability-based security system for the Lion WebAssembly Plugin System.
//! It provides a concrete implementation of the `CapabilityManager` trait from `lion_core`,
//! along with policy enforcement for various resource access types.
//!
//! ## Features
//! - Centralized capability management through the `CapabilityManagerImpl`
//! - Flexible policy system for controlling resource access
//! - Path-based filesystem access control
//! - Domain-based network access control
//! - Support for custom per-plugin policies

pub mod error;
pub mod manager;
pub mod policy;
pub mod checker;

// Re-exports for convenience
pub use error::CapabilityManagerError;
pub use manager::{CapabilityManagerImpl, CapabilityManagerImplConfig};
pub use policy::{CapabilityPolicy, DefaultCapabilityPolicy};
pub use checker::{check_capability, check_fs_read, check_fs_write, check_network, check_interplugin_comm};

/// Create a new capability manager with default configuration
pub fn create_default_manager() -> std::sync::Arc<dyn lion_core::capability::CapabilityManager> {
    std::sync::Arc::new(manager::CapabilityManagerImpl::new(
        manager::CapabilityManagerImplConfig::default()
    ))
}