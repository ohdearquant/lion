use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::AccessRequest;
use lion_core::id::PluginId;

/// An entry in the audit log
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Timestamp of the access (milliseconds since UNIX epoch)
    pub timestamp: u64,

    /// The plugin ID that made the access request
    pub plugin_id: PluginId,

    /// The access request that was checked
    pub request: AccessRequest,

    /// Whether the access was permitted
    pub permitted: bool,
}

impl AuditEntry {
    /// Creates a new audit entry
    pub fn new(plugin_id: &PluginId, request: &AccessRequest, permitted: bool) -> Self {
        // Get current time in milliseconds since epoch
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            timestamp: now,
            plugin_id: *plugin_id,
            request: request.clone(),
            permitted,
        }
    }
}

/// A thread-safe audit log for recording capability access checks
pub struct AuditLog {
    /// Map from plugin ID to list of audit entries
    entries: RwLock<HashMap<PluginId, Vec<AuditEntry>>>,

    /// Maximum number of entries per plugin
    max_entries_per_plugin: usize,
}

impl AuditLog {
    /// Creates a new audit log with the specified maximum entries per plugin
    pub fn new(max_entries_per_plugin: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries_per_plugin,
        }
    }

    /// Logs an access check
    pub fn log_access(&self, plugin_id: &PluginId, request: &AccessRequest, permitted: bool) {
        let entry = AuditEntry::new(plugin_id, request, permitted);

        // Add to the entries map
        let mut entries = self.entries.write().unwrap();

        let plugin_entries = entries.entry(*plugin_id).or_insert_with(Vec::new);

        // Add the new entry
        plugin_entries.push(entry);

        // Trim to max size if needed
        if plugin_entries.len() > self.max_entries_per_plugin {
            plugin_entries.drain(0..plugin_entries.len() - self.max_entries_per_plugin);
        }
    }

    /// Gets the audit entries for a plugin
    pub fn get_entries(&self, plugin_id: PluginId) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();

        entries
            .get(&plugin_id)
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// Gets all audit entries for all plugins
    pub fn get_all_entries(&self) -> HashMap<PluginId, Vec<AuditEntry>> {
        let entries = self.entries.read().unwrap();

        entries.clone()
    }

    /// Clears the audit entries for a plugin
    pub fn clear_entries(&self, plugin_id: PluginId) {
        let mut entries = self.entries.write().unwrap();

        entries.remove(&plugin_id);
    }

    /// Clears all audit entries
    pub fn clear_all_entries(&self) {
        let mut entries = self.entries.write().unwrap();

        entries.clear();
    }

    /// Gets the maximum number of entries per plugin
    pub fn max_entries_per_plugin(&self) -> usize {
        self.max_entries_per_plugin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log() {
        let audit_log = AuditLog::new(10);
        let plugin_id = PluginId::from_u64(1);

        // Log some accesses
        audit_log.log_access(
            &plugin_id,
            &AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            },
            true,
        );

        audit_log.log_access(
            &plugin_id,
            &AccessRequest::Network {
                host: Some("example.com".to_string()),
                port: None,
                connect: true,
                listen: false,
                bind: false,
            },
            false,
        );

        // Get the entries
        let entries = audit_log.get_entries(plugin_id);

        // Check the entries
        assert_eq!(entries.len(), 2);
        assert!(entries[0].permitted);
        assert!(!entries[1].permitted);

        // Clear the entries
        audit_log.clear_entries(plugin_id);

        // Check that the entries are gone
        let entries = audit_log.get_entries(plugin_id);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_audit_log_max_entries() {
        let audit_log = AuditLog::new(2);
        let plugin_id = PluginId::from_u64(1);

        // Log more than the maximum number of entries
        for i in 0..5 {
            audit_log.log_access(
                &plugin_id,
                &AccessRequest::File {
                    path: format!("/tmp/file{}.txt", i),
                    read: true,
                    write: false,
                    execute: false,
                },
                true,
            );
        }

        // Get the entries
        let entries = audit_log.get_entries(plugin_id);

        // Check that only the most recent entries are kept
        assert_eq!(entries.len(), 2);

        // Check the order
        match &entries[0].request {
            AccessRequest::File { path, .. } => assert_eq!(path, "/tmp/file3.txt"),
            _ => panic!("Unexpected request type"),
        }

        match &entries[1].request {
            AccessRequest::File { path, .. } => assert_eq!(path, "/tmp/file4.txt"),
            _ => panic!("Unexpected request type"),
        }
    }
}
