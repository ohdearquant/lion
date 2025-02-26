//! # Lion Capability
//!
//! This crate implements the Lion microkernel's capability-based security system.
//! It includes specialized capabilities (FileCapability, NetworkCapability, etc.),
//! partial revocation, composition, and a store for assigning capabilities to plugins.
//!
//! The crate follows a mathematical model where capabilities form a partial order
//! based on privilege inclusion. It provides efficient implementations of capability
//! operations including joins, meets, and constraints.
//!
//! ## Core Components
//!
//! - **Model**: Defines the capability types (File, Network, Memory, etc.)
//! - **Store**: Manages capabilities assigned to plugins, with support for partial revocation
//! - **Check**: Verifies that a plugin has permission for a specific access request
//! - **Attenuation**: Provides mechanisms for further restricting capabilities
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use std::collections::HashSet;
//! use std::sync::Arc;
//!
//! use lion_capability::model::{FileCapability, FileOperations, AccessRequest};
//! use lion_capability::store::InMemoryCapabilityStore;
//! use lion_capability::check::CapabilityChecker;
//! use lion_core::id::PluginId;
//!
//! // Create a capability store
//! let store = Arc::new(InMemoryCapabilityStore::new());
//! let plugin_id = PluginId::from_u64(1);
//!
//! // Create a file capability
//! let paths = ["/tmp/*".to_string()].into_iter().collect();
//! let file_cap = FileCapability::new(paths, FileOperations::READ);
//!
//! // Add the capability to the store
//! let capability_id = store.add_capability(plugin_id, Box::new(file_cap)).unwrap();
//!
//! // Create a capability checker
//! let checker = CapabilityChecker::new(store);
//!
//! // Check if the plugin has permission to read a file
//! let result = checker.check(&plugin_id, &AccessRequest::File {
//!     path: "/tmp/file.txt".to_string(),
//!     read: true,
//!     write: false,
//!     execute: false,
//! });
//!
//! assert!(result.is_ok());
//! ```

pub mod attenuation;
pub mod check;
pub mod model;
pub mod store;

// Re-export commonly used types
pub use model::{
    path_matches, AccessRequest, Capability, CapabilityError, CapabilityOwner, CompositeCapability,
    Constraint, FileCapability, MemoryCapability, MessageCapability, NetworkCapability,
    PluginCallCapability,
};

pub use attenuation::{CombineCapability, CombineStrategy, FilterCapability, ProxyCapability};
pub use check::{merge_capabilities_by_type, AuditLog, CapabilityChecker, CapabilitySet};
pub use store::{CapabilityRef, CapabilityStore, InMemoryCapabilityStore};
