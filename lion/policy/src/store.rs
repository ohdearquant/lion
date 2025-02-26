//! Policy storage.
//!
//! This module provides storage for policy rules,
//! enabling efficient lookup for enforcement.

use std::sync::Arc;
use std::collections::HashMap;

use dashmap::DashMap;
use parking_lot::RwLock;

use lion_core::error::PolicyError;
use lion_core::types::PluginId;
use crate::rule::PolicyRule;

/// Interface for policy storage.
pub trait PolicyStore: Send + Sync {
    /// Add a rule to the store.
    fn add_rule(&self, rule: Arc<dyn PolicyRule>) -> Result<(), PolicyError>;
    
    /// Remove a rule from the store.
    fn remove_rule(&self, rule_id: &str) -> Result<(), PolicyError>;
    
    /// Get all rules for file access.
    fn get_file_rules(&self, plugin_id: &PluginId) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError>;
    
    /// Get all rules for network access.
    fn get_network_rules(&self, plugin_id: &PluginId) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError>;
    
    /// Get all rules for resource usage.
    fn get_resource_rules(&self, plugin_id: &PluginId) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError>;
    
    /// Get all rules in the store.
    fn get_all_rules(&self) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError>;
}

/// In-memory policy store.
pub struct MemoryPolicyStore {
    /// All rules, indexed by ID.
    rules: DashMap<String, Arc<dyn PolicyRule>>,
    
    /// Rules by plugin ID (cached for faster lookup).
    plugin_rules: RwLock<HashMap<PluginId, Vec<String>>>,
    
    /// File access rules (cached for faster lookup).
    file_rules: RwLock<Vec<String>>,
    
    /// Network access rules (cached for faster lookup).
    network_rules: RwLock<Vec<String>>,
    
    /// Resource usage rules (cached for faster lookup).
    resource_rules: RwLock<Vec<String>>,
}

impl MemoryPolicyStore {
    /// Create a new in-memory policy store.
    pub fn new() -> Self {
        Self {
            rules: DashMap::new(),
            plugin_rules: RwLock::new(HashMap::new()),
            file_rules: RwLock::new(Vec::new()),
            network_rules: RwLock::new(Vec::new()),
            resource_rules: RwLock::new(Vec::new()),
        }
    }
    
    /// Categorize a rule based on a test plugin evaluation.
    fn categorize_rule(&self, rule: &Arc<dyn PolicyRule>) {
        // Create a dummy plugin ID
        let dummy_id = PluginId::new();
        
        // Check if the rule applies to files
        let dummy_path = std::path::Path::new("/dummy/path");
        if rule.evaluate_file_access(&dummy_id, dummy_path, false).is_some() {
            // This is a file rule
            let mut file_rules = self.file_rules.write();
            file_rules.push(rule.get_id().to_string());
        }
        
        // Check if the rule applies to network
        if rule.evaluate_network_access(&dummy_id, "example.com", 80, false).is_some() {
            // This is a network rule
            let mut network_rules = self.network_rules.write();
            network_rules.push(rule.get_id().to_string());
        }
        
        // Check if the rule applies to resources
        if rule.evaluate_resource_usage(&dummy_id, "memory", 1000).is_some() {
            // This is a resource rule
            let mut resource_rules = self.resource_rules.write();
            resource_rules.push(rule.get_id().to_string());
        }
    }
}

impl PolicyStore for MemoryPolicyStore {
    fn add_rule(&self, rule: Arc<dyn PolicyRule>) -> Result<(), PolicyError> {
        let rule_id = rule.get_id().to_string();
        
        // Check if the rule already exists
        if self.rules.contains_key(&rule_id) {
            return Err(PolicyError::EvaluationFailed(
                format!("Rule with ID {} already exists", rule_id)
            ));
        }
        
        // Categorize the rule
        self.categorize_rule(&rule);
        
        // Add the rule
        self.rules.insert(rule_id, rule);
        
        Ok(())
    }
    
    fn remove_rule(&self, rule_id: &str) -> Result<(), PolicyError> {
        // Remove the rule
        if self.rules.remove(rule_id).is_none() {
            return Err(PolicyError::EvaluationFailed(
                format!("Rule with ID {} does not exist", rule_id)
            ));
        }
        
        // Update caches
        {
            let mut file_rules = self.file_rules.write();
            file_rules.retain(|id| id != rule_id);
        }
        
        {
            let mut network_rules = self.network_rules.write();
            network_rules.retain(|id| id != rule_id);
        }
        
        {
            let mut resource_rules = self.resource_rules.write();
            resource_rules.retain(|id| id != rule_id);
        }
        
        {
            let mut plugin_rules = self.plugin_rules.write();
            for rules in plugin_rules.values_mut() {
                rules.retain(|id| id != rule_id);
            }
        }
        
        Ok(())
    }
    
    fn get_file_rules(&self, _plugin_id: &PluginId) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError> {
        let file_rules = self.file_rules.read();
        let mut result = Vec::new();
        
        for rule_id in file_rules.iter() {
            if let Some(rule) = self.rules.get(rule_id) {
                result.push(rule.clone());
            }
        }
        
        Ok(result)
    }
    
    fn get_network_rules(&self, _plugin_id: &PluginId) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError> {
        let network_rules = self.network_rules.read();
        let mut result = Vec::new();
        
        for rule_id in network_rules.iter() {
            if let Some(rule) = self.rules.get(rule_id) {
                result.push(rule.clone());
            }
        }
        
        Ok(result)
    }
    
    fn get_resource_rules(&self, _plugin_id: &PluginId) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError> {
        let resource_rules = self.resource_rules.read();
        let mut result = Vec::new();
        
        for rule_id in resource_rules.iter() {
            if let Some(rule) = self.rules.get(rule_id) {
                result.push(rule.clone());
            }
        }
        
        Ok(result)
    }
    
    fn get_all_rules(&self) -> Result<Vec<Arc<dyn PolicyRule>>, PolicyError> {
        let mut result = Vec::new();
        
        for entry in self.rules.iter() {
            result.push(entry.value().clone());
        }
        
        Ok(result)
    }
}