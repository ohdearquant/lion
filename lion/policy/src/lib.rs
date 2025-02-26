//! Lion Policy - Enforcement rules for the Lion runtime
//!
//! This crate provides a policy system for enforcing rules
//! across the Lion runtime. It manages cross-cutting concerns
//! like resource limits, network access, file permissions, and more.

mod engine;
mod rule;
mod store;

pub use engine::{PolicyEngine, PolicyEvaluator};
pub use rule::{PolicyRule, PolicyAction, ResourceLimitRule, NetworkRule, FileRule};
pub use store::{PolicyStore, MemoryPolicyStore};

use std::sync::Arc;
use dashmap::DashMap;

use lion_core::error::{PolicyError, Result};
use lion_core::types::PluginId;
use lion_capabilities::CapabilityManager;

/// Core policy manager that integrates storage and enforcement.
pub struct PolicyManager {
    /// Policy storage.
    store: Arc<dyn PolicyStore>,
    
    /// Policy engine.
    engine: Arc<dyn PolicyEngine>,
    
    /// Capability manager for checking capabilities.
    capability_manager: Arc<CapabilityManager>,
    
    /// Audit log for policy decisions.
    audit_log: DashMap<u64, PolicyAudit>,
    
    /// Next audit ID.
    next_audit_id: std::sync::atomic::AtomicU64,
}

/// Audit entry for policy decisions.
#[derive(Debug, Clone)]
pub struct PolicyAudit {
    /// Unique audit ID.
    pub id: u64,
    
    /// Plugin ID that was checked.
    pub plugin_id: PluginId,
    
    /// Resource or object being accessed.
    pub resource: String,
    
    /// Action being performed.
    pub action: String,
    
    /// Whether the action was allowed.
    pub allowed: bool,
    
    /// Policy rule that made the decision.
    pub rule_id: Option<String>,
    
    /// Additional details.
    pub details: String,
    
    /// When the decision was made.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PolicyManager {
    /// Create a new policy manager.
    pub fn new(
        store: Arc<dyn PolicyStore>,
        engine: Arc<dyn PolicyEngine>,
        capability_manager: Arc<CapabilityManager>,
    ) -> Self {
        Self {
            store,
            engine,
            capability_manager,
            audit_log: DashMap::new(),
            next_audit_id: std::sync::atomic::AtomicU64::new(1),
        }
    }
    
    /// Get a simple default implementation.
    pub fn default(capability_manager: Arc<CapabilityManager>) -> Self {
        let store = Arc::new(MemoryPolicyStore::new());
        let engine = Arc::new(PolicyEvaluator::new());
        Self::new(store, engine, capability_manager)
    }
    
    /// Check if a plugin can access a file.
    pub fn check_file_access(
        &self,
        plugin_id: &PluginId,
        path: &std::path::Path,
        write: bool,
    ) -> Result<(), PolicyError> {
        // Create a record for the audit log
        let audit_id = self.record_decision(
            plugin_id,
            &format!("file:{}", path.display()),
            if write { "write" } else { "read" },
            None,
        );
        
        // Check if the plugin has file capability
        let mut file_cap = lion_capabilities::FileCapability::new(!write, write);
        file_cap.add_path(path.to_path_buf());
        
        // First check if the plugin has the capability
        let cap_result = self.capability_manager.check_capability(
            plugin_id,
            &file_cap,
        );
        
        // If capability check fails, deny immediately
        if let Err(e) = cap_result {
            // Update the audit record
            if let Some(mut entry) = self.audit_log.get_mut(&audit_id) {
                entry.allowed = false;
                entry.details = format!("Capability denied: {}", e);
            }
            
            return Err(PolicyError::FileAccessViolation(
                format!("Capability check failed: {}", e)
            ));
        }
        
        // Now check policy rules
        let rules = self.store.get_file_rules(plugin_id)?;
        let result = self.engine.evaluate_file_access(plugin_id, path, write, &rules);
        
        // Update the audit record with the result
        if let Some(mut entry) = self.audit_log.get_mut(&audit_id) {
            entry.allowed = result.is_ok();
            if let Err(ref e) = result {
                entry.details = format!("Policy denied: {}", e);
            } else {
                entry.details = "Allowed by policy".to_string();
            }
            
            // Add rule ID if available
            if let Ok(rule_id) = self.engine.get_matched_rule_id() {
                entry.rule_id = Some(rule_id);
            }
        }
        
        result
    }
    
    /// Check if a plugin can access the network.
    pub fn check_network_access(
        &self,
        plugin_id: &PluginId,
        host: &str,
        port: u16,
        listen: bool,
    ) -> Result<(), PolicyError> {
        // Create a record for the audit log
        let audit_id = self.record_decision(
            plugin_id,
            &format!("network:{}:{}", host, port),
            if listen { "listen" } else { "connect" },
            None,
        );
        
        // Check if the plugin has network capability
        let mut net_cap = lion_capabilities::NetworkCapability::new(!listen, listen);
        net_cap.add_host(host.to_string());
        if listen {
            net_cap.add_port(port);
        }
        
        // First check if the plugin has the capability
        let cap_result = self.capability_manager.check_capability(
            plugin_id,
            &net_cap,
        );
        
        // If capability check fails, deny immediately
        if let Err(e) = cap_result {
            // Update the audit record
            if let Some(mut entry) = self.audit_log.get_mut(&audit_id) {
                entry.allowed = false;
                entry.details = format!("Capability denied: {}", e);
            }
            
            return Err(PolicyError::NetworkAccessViolation(
                format!("Capability check failed: {}", e)
            ));
        }
        
        // Now check policy rules
        let rules = self.store.get_network_rules(plugin_id)?;
        let result = self.engine.evaluate_network_access(plugin_id, host, port, listen, &rules);
        
        // Update the audit record with the result
        if let Some(mut entry) = self.audit_log.get_mut(&audit_id) {
            entry.allowed = result.is_ok();
            if let Err(ref e) = result {
                entry.details = format!("Policy denied: {}", e);
            } else {
                entry.details = "Allowed by policy".to_string();
            }
            
            // Add rule ID if available
            if let Ok(rule_id) = self.engine.get_matched_rule_id() {
                entry.rule_id = Some(rule_id);
            }
        }
        
        result
    }
    
    /// Check if a plugin can use resources (memory, CPU).
    pub fn check_resource_usage(
        &self,
        plugin_id: &PluginId,
        resource_type: &str,
        amount: u64,
    ) -> Result<(), PolicyError> {
        // Create a record for the audit log
        let audit_id = self.record_decision(
            plugin_id,
            &format!("resource:{}", resource_type),
            &format!("use:{}", amount),
            None,
        );
        
        // Check policy rules
        let rules = self.store.get_resource_rules(plugin_id)?;
        let result = self.engine.evaluate_resource_usage(plugin_id, resource_type, amount, &rules);
        
        // Update the audit record with the result
        if let Some(mut entry) = self.audit_log.get_mut(&audit_id) {
            entry.allowed = result.is_ok();
            if let Err(ref e) = result {
                entry.details = format!("Policy denied: {}", e);
            } else {
                entry.details = "Allowed by policy".to_string();
            }
            
            // Add rule ID if available
            if let Ok(rule_id) = self.engine.get_matched_rule_id() {
                entry.rule_id = Some(rule_id);
            }
        }
        
        result
    }
    
    /// Record a policy decision in the audit log.
    fn record_decision(
        &self,
        plugin_id: &PluginId,
        resource: &str,
        action: &str,
        details: Option<String>,
    ) -> u64 {
        let id = self.next_audit_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let entry = PolicyAudit {
            id,
            plugin_id: plugin_id.clone(),
            resource: resource.to_string(),
            action: action.to_string(),
            allowed: true, // Will be updated later
            rule_id: None, // Will be updated later
            details: details.unwrap_or_else(|| "Policy evaluation pending".to_string()),
            timestamp: chrono::Utc::now(),
        };
        
        self.audit_log.insert(id, entry);
        id
    }
    
    /// Get recent policy decisions.
    pub fn get_recent_decisions(&self, limit: usize) -> Vec<PolicyAudit> {
        let mut entries: Vec<PolicyAudit> = self.audit_log.iter()
            .map(|entry| entry.value().clone())
            .collect();
        
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        entries.truncate(limit);
        
        entries
    }
    
    /// Get policy decisions for a specific plugin.
    pub fn get_plugin_decisions(&self, plugin_id: &PluginId, limit: usize) -> Vec<PolicyAudit> {
        let mut entries: Vec<PolicyAudit> = self.audit_log.iter()
            .map(|entry| entry.value().clone())
            .filter(|entry| entry.plugin_id == *plugin_id)
            .collect();
        
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        entries.truncate(limit);
        
        entries
    }
    
    /// Add a rule to the policy store.
    pub fn add_rule(&self, rule: Arc<dyn PolicyRule>) -> Result<(), PolicyError> {
        self.store.add_rule(rule)
    }
    
    /// Remove a rule from the policy store.
    pub fn remove_rule(&self, rule_id: &str) -> Result<(), PolicyError> {
        self.store.remove_rule(rule_id)
    }
    
    /// Get all rules.
    pub fn get_rules(&self) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError> {
        self.store.get_all_rules()
    }
}