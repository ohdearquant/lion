//! Default capability checker.
//!
//! This module provides a default implementation of the capability checker.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::interface::CapabilityChecker;

/// A default capability checker.
///
/// This checker uses a simple in-memory map of allowed operations.
/// For a real implementation, you would likely integrate with a more
/// sophisticated capability system like `lion_capability`.
pub struct DefaultCapabilityChecker {
    /// The allowed operations, keyed by plugin ID.
    allowed_operations: RwLock<HashMap<String, Vec<String>>>,
}

impl DefaultCapabilityChecker {
    /// Create a new default capability checker.
    pub fn new() -> Self {
        Self {
            allowed_operations: RwLock::new(HashMap::new()),
        }
    }

    /// Grant a capability to a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `operation` - The operation.
    pub fn grant_capability(&self, plugin_id: &str, operation: &str) {
        let mut map = self.allowed_operations.write().unwrap();

        let operations = map.entry(plugin_id.to_string()).or_insert_with(Vec::new);

        if !operations.contains(&operation.to_string()) {
            operations.push(operation.to_string());
        }
    }

    /// Revoke a capability from a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `operation` - The operation.
    pub fn revoke_capability(&self, plugin_id: &str, operation: &str) {
        let mut map = self.allowed_operations.write().unwrap();

        if let Some(operations) = map.get_mut(plugin_id) {
            if let Some(index) = operations.iter().position(|op| op == operation) {
                operations.remove(index);
            }
        }
    }
}

impl CapabilityChecker for DefaultCapabilityChecker {
    fn check_capability(&self, plugin_id: &str, operation: &str, _params: &[u8]) -> Result<()> {
        let map = self.allowed_operations.read().unwrap();

        if let Some(operations) = map.get(plugin_id) {
            if operations.contains(&operation.to_string()) {
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "Operation '{}' not allowed for plugin '{}'",
            operation,
            plugin_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_capability_checker() {
        let checker = DefaultCapabilityChecker::new();

        // Check that operations are not allowed by default
        assert!(checker.check_capability("test", "read_file", &[]).is_err());

        // Grant a capability
        checker.grant_capability("test", "read_file");

        // Check that the operation is now allowed
        assert!(checker.check_capability("test", "read_file", &[]).is_ok());

        // Check that other operations are still not allowed
        assert!(checker.check_capability("test", "write_file", &[]).is_err());

        // Revoke the capability
        checker.revoke_capability("test", "read_file");

        // Check that the operation is no longer allowed
        assert!(checker.check_capability("test", "read_file", &[]).is_err());
    }
}
