//! Capability policy enforcement system.
//!
//! This module defines policy types that control which capabilities
//! can be granted to which plugins.

mod default_policy;
mod fs_policy;
mod network_policy;

pub use default_policy::DefaultCapabilityPolicy;
pub use fs_policy::FilesystemPolicy;
pub use network_policy::NetworkPolicy;

use lion_core::capability::CoreCapability;
use lion_core::plugin::PluginId;

/// A policy that controls capability grants
pub trait CapabilityPolicy: Send + Sync {
    /// Check if a capability can be granted to a plugin
    fn can_grant_capability(
        &self,
        plugin_id: PluginId,
        capability: &CoreCapability,
    ) -> bool;
    
    /// Get the reason a capability cannot be granted
    fn get_denial_reason(
        &self,
        plugin_id: PluginId,
        capability: &CoreCapability,
    ) -> Option<String>;
}