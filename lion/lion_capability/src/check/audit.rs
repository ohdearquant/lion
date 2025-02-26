//! Capability auditing.
//! 
//! This module provides capability auditing functionality.

use std::sync::Arc;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use lion_core::error::Result;
use lion_core::id::PluginId;
use lion_core::types::AccessRequest;

/// An audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// When the access was attempted.
    pub timestamp: DateTime<Utc>,
    
    /// The ID of the plugin that attempted the access.
    pub plugin_id: PluginId,
    
    /// The access request.
    pub request: AccessRequest,
    
    /// Whether the access was permitted.
    pub permitted: bool,
}

/// An audit log.
#[derive(Clone)]
pub struct AuditLog {
    /// The audit entries.
    entries: Arc<DashMap<PluginId, Vec<AuditEntry>>>,
    
    /// The maximum number of entries to keep per plugin.
    max_entries_per_plugin: usize,
}

impl AuditLog {
    /// Create a new audit log.
    ///
    /// # Arguments
    ///
    /// * `max_entries_per_plugin` - The maximum number of entries to keep per plugin.
    ///
    /// # Returns
    ///
    /// A new audit log.
    pub fn new(max_entries_per_plugin: usize) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_entries_per_plugin,
        }
    }
    
    /// Log an access attempt.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin that attempted the access.
    /// * `request` - The access request.
    /// * `permitted` - Whether the access was permitted.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the access was successfully logged.
    /// * `Err` - If the access could not be logged.
    pub fn log_access(
        &self,
        plugin_id: &PluginId,
        request: &AccessRequest,
        permitted: bool,
    ) -> Result<()> {
        // Create an entry
        let entry = AuditEntry {
            timestamp: Utc::now(),
            plugin_id: plugin_id.clone(),
            request: request.clone(),
            permitted,
        };
        
        // Add the entry to the log
        self.entries
            .entry(plugin_id.clone())
            .or_insert_with(Vec::new)
            .push(entry);
        
        // Trim the log if necessary
        if let Some(mut plugin_entries) = self.entries.get_mut(plugin_id) {
            if plugin_entries.len() > self.max_entries_per_plugin {
                let to_remove = plugin_entries.len() - self.max_entries_per_plugin;
                plugin_entries.drain(0..to_remove);
            }
        }
        
        Ok(())
    }
    
    /// Get the audit entries for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get entries for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<AuditEntry>)` - The audit entries.
    /// * `Err` - If the entries could not be retrieved.
    pub fn get_entries(&self, plugin_id: &PluginId) -> Result<Vec<AuditEntry>> {
        // Get the entries for the plugin
        let entries = match self.entries.get(plugin_id) {
            Some(entries) => entries.clone(),
            None => Vec::new(),
        };
        
        Ok(entries)
    }
    
    /// Clear the audit entries for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to clear entries for.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the entries were successfully cleared.
    /// * `Err` - If the entries could not be cleared.
    pub fn clear_entries(&self, plugin_id: &PluginId) -> Result<()> {
        // Clear the entries for the plugin
        self.entries.remove(plugin_id);
        
        Ok(())
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_log_and_get_entries() {
        let audit_log = AuditLog::new(10);
        let plugin_id = PluginId::new();
        
        // Log an access attempt
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        audit_log.log_access(&plugin_id, &request, true).unwrap();
        
        // Get the entries
        let entries = audit_log.get_entries(&plugin_id).unwrap();
        
        // Check that there's one entry
        assert_eq!(entries.len(), 1);
        
        // Check the entry details
        let entry = &entries[0];
        assert_eq!(entry.plugin_id, plugin_id);
        assert!(entry.permitted);
        
        match &entry.request {
            AccessRequest::File { path, read, write, execute } => {
                assert_eq!(path, &PathBuf::from("/tmp/file"));
                assert!(read);
                assert!(!write);
                assert!(!execute);
            },
            _ => panic!("Unexpected request type"),
        }
    }
    
    #[test]
    fn test_clear_entries() {
        let audit_log = AuditLog::new(10);
        let plugin_id = PluginId::new();
        
        // Log an access attempt
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        audit_log.log_access(&plugin_id, &request, true).unwrap();
        
        // Clear the entries
        audit_log.clear_entries(&plugin_id).unwrap();
        
        // Get the entries
        let entries = audit_log.get_entries(&plugin_id).unwrap();
        
        // Check that there are no entries
        assert_eq!(entries.len(), 0);
    }
    
    #[test]
    fn test_max_entries_per_plugin() {
        let audit_log = AuditLog::new(2);
        let plugin_id = PluginId::new();
        
        // Log three access attempts
        for i in 0..3 {
            let request = AccessRequest::File {
                path: PathBuf::from(format!("/tmp/file{}", i)),
                read: true,
                write: false,
                execute: false,
            };
            audit_log.log_access(&plugin_id, &request, true).unwrap();
        }
        
        // Get the entries
        let entries = audit_log.get_entries(&plugin_id).unwrap();
        
        // Check that there are only two entries
        assert_eq!(entries.len(), 2);
        
        // Check that the oldest entry was removed
        match &entries[0].request {
            AccessRequest::File { path, .. } => {
                assert_eq!(path, &PathBuf::from("/tmp/file1"));
            },
            _ => panic!("Unexpected request type"),
        }
    }
}