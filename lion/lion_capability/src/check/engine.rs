use std::sync::Arc;

use lion_core::id::PluginId;

use super::audit::AuditLog;
use crate::model::{AccessRequest, CapabilityError};
use crate::store::CapabilityStore;

/// The main capability checking engine
///
/// This component is responsible for verifying that a plugin has permission
/// to perform a specific action by checking its assigned capabilities.
pub struct CapabilityChecker {
    /// The capability store to use for lookups
    store: Arc<dyn CapabilityStore>,

    /// Optional audit log for recording capability checks
    audit_log: Option<Arc<AuditLog>>,
}

impl CapabilityChecker {
    /// Creates a new capability checker with the specified store
    pub fn new(store: Arc<dyn CapabilityStore>) -> Self {
        Self {
            store,
            audit_log: None,
        }
    }

    /// Creates a new capability checker with the specified store and audit log
    pub fn with_audit(store: Arc<dyn CapabilityStore>, audit_log: Arc<AuditLog>) -> Self {
        Self {
            store,
            audit_log: Some(audit_log),
        }
    }

    /// Sets the audit log for this checker
    pub fn set_audit_log(&mut self, audit_log: Option<Arc<AuditLog>>) {
        self.audit_log = audit_log;
    }

    /// Checks if a plugin has permission for a specific access request
    ///
    /// This method delegates to the capability store to check if any of the
    /// plugin's capabilities permit the request, and records the check in
    /// the audit log if one is configured.
    pub fn check(
        &self,
        plugin_id: &PluginId,
        request: &AccessRequest,
    ) -> Result<(), CapabilityError> {
        // Check if the plugin has permission
        let result = self.store.check_permission(plugin_id, request);

        // Record in the audit log if available
        if let Some(audit_log) = &self.audit_log {
            audit_log.log_access(plugin_id, request, result.is_ok());
        }

        result
    }

    /// Gets a reference to the capability store
    pub fn store(&self) -> &Arc<dyn CapabilityStore> {
        &self.store
    }

    /// Gets a reference to the audit log, if one is configured
    pub fn audit_log(&self) -> Option<&Arc<AuditLog>> {
        self.audit_log.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    use crate::model::file::{FileCapability, FileOperations};
    use crate::store::InMemoryCapabilityStore;

    fn test_plugin_id(value: u64) -> PluginId {
        // Create a deterministic UUID from the value
        let bytes = value.to_le_bytes();
        let mut uuid_bytes = [0u8; 16];
        for i in 0..std::cmp::min(8, uuid_bytes.len()) {
            uuid_bytes[i] = bytes[i];
        }
        let uuid = Uuid::from_bytes(uuid_bytes);
        PluginId::from_uuid(uuid)
    }

    #[test]
    fn test_capability_checker() {
        // Create a store
        let store = Arc::new(InMemoryCapabilityStore::new());
        let plugin_id = test_plugin_id(1);

        // Add a capability to the store
        let paths = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        store.add_capability(plugin_id, Box::new(file_cap)).unwrap();

        // Create a checker
        let checker = CapabilityChecker::new(store);

        // Check a permitted request
        assert!(checker
            .check(
                &plugin_id,
                &AccessRequest::File {
                    path: "/tmp/file.txt".to_string(),
                    read: true,
                    write: false,
                    execute: false,
                }
            )
            .is_ok());

        // Check a denied request
        assert!(checker
            .check(
                &plugin_id,
                &AccessRequest::File {
                    path: "/tmp/file.txt".to_string(),
                    read: false,
                    write: true,
                    execute: false,
                }
            )
            .is_err());
    }

    #[test]
    fn test_capability_checker_with_audit() {
        // Create a store
        let store = Arc::new(InMemoryCapabilityStore::new());
        let plugin_id = test_plugin_id(1);

        // Add a capability to the store
        let paths = ["/tmp/file.txt".to_string()].into_iter().collect();
        let file_cap = FileCapability::new(paths, FileOperations::READ);

        store.add_capability(plugin_id, Box::new(file_cap)).unwrap();

        // Create an audit log
        let audit_log = Arc::new(AuditLog::new(100));

        // Create a checker with audit
        let checker = CapabilityChecker::with_audit(store, audit_log.clone());

        // Make some checks
        checker
            .check(
                &plugin_id,
                &AccessRequest::File {
                    path: "/tmp/file.txt".to_string(),
                    read: true,
                    write: false,
                    execute: false,
                },
            )
            .ok();

        checker
            .check(
                &plugin_id,
                &AccessRequest::File {
                    path: "/tmp/file.txt".to_string(),
                    read: false,
                    write: true,
                    execute: false,
                },
            )
            .ok();

        // Check the audit log
        let entries = audit_log.get_entries(plugin_id);

        assert_eq!(entries.len(), 2);
        assert!(entries[0].permitted);
        assert!(!entries[1].permitted);
    }
}
