//! Lion Capabilities - Security primitives for the Lion plugin system
//!
//! This crate provides capability-based security for Lion plugins,
//! enabling fine-grained access control that follows the principle
//! of least privilege.

mod capability;
mod checker;
mod store;

pub use capability::{
    Capability, CapabilitySet, CapabilityType,
    FileCapability, NetworkCapability, MessageCapability, PluginCallCapability,
};
pub use checker::{CapabilityChecker, SimpleCapabilityChecker};
pub use store::{CapabilityStore, MemoryCapabilityStore};

use std::sync::Arc;
use dashmap::DashMap;

use lion_core::error::{CapabilityError, Result};
use lion_core::types::PluginId;

/// Audit entry for capability checks.
#[derive(Debug, Clone)]
pub struct CapabilityAudit {
    /// Unique audit ID.
    pub id: u64,
    
    /// Plugin ID that was checked.
    pub plugin_id: PluginId,
    
    /// Capability type that was checked.
    pub capability_type: String,
    
    /// Whether the check was successful.
    pub allowed: bool,
    
    /// Additional details about the check.
    pub details: String,
    
    /// When the check occurred.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Core capability manager that integrates storage and checking.
pub struct CapabilityManager {
    /// Storage for capability assignments.
    store: Arc<dyn CapabilityStore>,
    
    /// Checker for capability enforcement.
    checker: Arc<dyn CapabilityChecker>,
    
    /// Audit log.
    audit_log: DashMap<u64, CapabilityAudit>,
    
    /// Next audit ID.
    next_audit_id: std::sync::atomic::AtomicU64,
}

impl CapabilityManager {
    /// Create a new capability manager.
    pub fn new(
        store: Arc<dyn CapabilityStore>,
        checker: Arc<dyn CapabilityChecker>,
    ) -> Self {
        Self {
            store,
            checker,
            audit_log: DashMap::new(),
            next_audit_id: std::sync::atomic::AtomicU64::new(1),
        }
    }
    
    /// Get a simple default implementation.
    pub fn default() -> Self {
        let store = Arc::new(MemoryCapabilityStore::new());
        let checker = Arc::new(SimpleCapabilityChecker::new());
        Self::new(store, checker)
    }
    
    /// Check if a plugin has a capability.
    pub fn check_capability(
        &self,
        plugin_id: &PluginId,
        capability: &dyn Capability,
    ) -> Result<(), CapabilityError> {
        // Record the check attempt
        let audit_id = self.record_check(plugin_id, capability, None);
        
        // Perform the check
        let result = self.checker.check_capability(
            plugin_id,
            capability,
            &self.store,
        );
        
        // Update the audit record with the result
        if let Some(mut entry) = self.audit_log.get_mut(&audit_id) {
            entry.allowed = result.is_ok();
            if let Err(ref e) = result {
                entry.details = format!("Denied: {}", e);
            } else {
                entry.details = "Allowed".to_string();
            }
        }
        
        result
    }
    
    /// Grant a capability to a plugin.
    pub fn grant_capability(
        &self,
        plugin_id: &PluginId,
        capability: Arc<dyn Capability>,
    ) -> Result<(), CapabilityError> {
        self.store.add_capability(plugin_id, capability.clone())?;
        
        // Record the grant in the audit log
        self.record_check(
            plugin_id,
            capability.as_ref(),
            Some(format!("Granted capability: {}", capability.get_type())),
        );
        
        Ok(())
    }
    
    /// Revoke a capability from a plugin.
    pub fn revoke_capability(
        &self,
        plugin_id: &PluginId,
        capability_type: &str,
    ) -> Result<(), CapabilityError> {
        // Try to revoke the capability
        let result = self.store.remove_capability(plugin_id, capability_type);
        
        // Record the revocation attempt
        let details = if result.is_ok() {
            format!("Revoked capability: {}", capability_type)
        } else {
            format!("Failed to revoke capability: {}", capability_type)
        };
        
        self.record_check_simple(plugin_id, capability_type, result.is_ok(), details);
        
        result
    }
    
    /// Get all capabilities for a plugin.
    pub fn get_capabilities(&self, plugin_id: &PluginId) -> Result<CapabilitySet, CapabilityError> {
        self.store.get_capabilities(plugin_id)
    }
    
    /// Record a capability check in the audit log.
    fn record_check(
        &self,
        plugin_id: &PluginId,
        capability: &dyn Capability,
        details: Option<String>,
    ) -> u64 {
        let id = self.next_audit_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let entry = CapabilityAudit {
            id,
            plugin_id: plugin_id.clone(),
            capability_type: capability.get_type().to_string(),
            allowed: true, // Will be updated later
            details: details.unwrap_or_else(|| "Check pending".to_string()),
            timestamp: chrono::Utc::now(),
        };
        
        self.audit_log.insert(id, entry);
        id
    }
    
    /// Record a simpler capability check in the audit log.
    fn record_check_simple(
        &self,
        plugin_id: &PluginId,
        capability_type: &str,
        allowed: bool,
        details: String,
    ) -> u64 {
        let id = self.next_audit_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let entry = CapabilityAudit {
            id,
            plugin_id: plugin_id.clone(),
            capability_type: capability_type.to_string(),
            allowed,
            details,
            timestamp: chrono::Utc::now(),
        };
        
        self.audit_log.insert(id, entry);
        id
    }
    
    /// Get recent audit entries.
    pub fn get_recent_audits(&self, limit: usize) -> Vec<CapabilityAudit> {
        let mut entries: Vec<CapabilityAudit> = self.audit_log.iter()
            .map(|entry| entry.value().clone())
            .collect();
        
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        entries.truncate(limit);
        
        entries
    }
    
    /// Get audit entries for a specific plugin.
    pub fn get_plugin_audits(&self, plugin_id: &PluginId, limit: usize) -> Vec<CapabilityAudit> {
        let mut entries: Vec<CapabilityAudit> = self.audit_log.iter()
            .map(|entry| entry.value().clone())
            .filter(|entry| entry.plugin_id == *plugin_id)
            .collect();
        
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        entries.truncate(limit);
        
        entries
    }
}