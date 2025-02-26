//! Policy evaluation engine.
//!
//! This module provides the core logic for evaluating policies
//! against actions and resources.

use std::path::Path;
use std::sync::Mutex;
use std::sync::Arc;

use lion_core::error::PolicyError;
use lion_core::types::PluginId;
use crate::rule::{PolicyRule, PolicyAction};

/// Interface for policy evaluation.
pub trait PolicyEngine: Send + Sync {
    /// Evaluate file access.
    fn evaluate_file_access(
        &self,
        plugin_id: &PluginId,
        path: &Path,
        write: bool,
        rules: &[Arc<dyn PolicyRule>],
    ) -> Result<(), PolicyError>;
    
    /// Evaluate network access.
    fn evaluate_network_access(
        &self,
        plugin_id: &PluginId,
        host: &str,
        port: u16,
        listen: bool,
        rules: &[Arc<dyn PolicyRule>],
    ) -> Result<(), PolicyError>;
    
    /// Evaluate resource usage.
    fn evaluate_resource_usage(
        &self,
        plugin_id: &PluginId,
        resource_type: &str,
        amount: u64,
        rules: &[Arc<dyn PolicyRule>],
    ) -> Result<(), PolicyError>;
    
    /// Get the ID of the rule that matched last.
    fn get_matched_rule_id(&self) -> Result<String, PolicyError>;
}

/// Default policy evaluator.
pub struct PolicyEvaluator {
    /// The last rule that matched.
    last_matched_rule: Mutex<Option<String>>,
}

impl PolicyEvaluator {
    /// Create a new policy evaluator.
    pub fn new() -> Self {
        Self {
            last_matched_rule: Mutex::new(None),
        }
    }
    
    /// Set the last matched rule.
    fn set_matched_rule(&self, rule_id: &str) {
        let mut last_rule = self.last_matched_rule.lock().unwrap();
        *last_rule = Some(rule_id.to_string());
    }
}

impl PolicyEngine for PolicyEvaluator {
    fn evaluate_file_access(
        &self,
        plugin_id: &PluginId,
        path: &Path,
        write: bool,
        rules: &[Arc<dyn PolicyRule>],
    ) -> Result<(), PolicyError> {
        // Check each rule in order
        for rule in rules {
            if let Some(action) = rule.evaluate_file_access(plugin_id, path, write) {
                // Record the matched rule
                self.set_matched_rule(rule.get_id());
                
                // Apply the action
                match action {
                    PolicyAction::Allow => return Ok(()),
                    PolicyAction::Deny(reason) => return Err(PolicyError::FileAccessViolation(reason)),
                    PolicyAction::Log(message) => {
                        // Log the message
                        log::info!("Policy log: {}", message);
                        // Allow by default after logging
                        return Ok(());
                    },
                }
            }
        }
        
        // If no rule matched, deny by default
        Err(PolicyError::FileAccessViolation(
            format!("No policy rule matched for file access to {}", path.display())
        ))
    }
    
    fn evaluate_network_access(
        &self,
        plugin_id: &PluginId,
        host: &str,
        port: u16,
        listen: bool,
        rules: &[Arc<dyn PolicyRule>],
    ) -> Result<(), PolicyError> {
        // Check each rule in order
        for rule in rules {
            if let Some(action) = rule.evaluate_network_access(plugin_id, host, port, listen) {
                // Record the matched rule
                self.set_matched_rule(rule.get_id());
                
                // Apply the action
                match action {
                    PolicyAction::Allow => return Ok(()),
                    PolicyAction::Deny(reason) => return Err(PolicyError::NetworkAccessViolation(reason)),
                    PolicyAction::Log(message) => {
                        // Log the message
                        log::info!("Policy log: {}", message);
                        // Allow by default after logging
                        return Ok(());
                    },
                }
            }
        }
        
        // If no rule matched, deny by default
        Err(PolicyError::NetworkAccessViolation(
            format!("No policy rule matched for network access to {}:{}", host, port)
        ))
    }
    
    fn evaluate_resource_usage(
        &self,
        plugin_id: &PluginId,
        resource_type: &str,
        amount: u64,
        rules: &[Arc<dyn PolicyRule>],
    ) -> Result<(), PolicyError> {
        // Check each rule in order
        for rule in rules {
            if let Some(action) = rule.evaluate_resource_usage(plugin_id, resource_type, amount) {
                // Record the matched rule
                self.set_matched_rule(rule.get_id());
                
                // Apply the action
                match action {
                    PolicyAction::Allow => return Ok(()),
                    PolicyAction::Deny(reason) => return Err(PolicyError::ResourceLimitExceeded(reason)),
                    PolicyAction::Log(message) => {
                        // Log the message
                        log::info!("Policy log: {}", message);
                        // Allow by default after logging
                        return Ok(());
                    },
                }
            }
        }
        
        // If no rule matched, allow by default for resources
        Ok(())
    }
    
    fn get_matched_rule_id(&self) -> Result<String, PolicyError> {
        let last_rule = self.last_matched_rule.lock().unwrap();
        if let Some(ref rule_id) = *last_rule {
            Ok(rule_id.clone())
        } else {
            Err(PolicyError::EvaluationFailed("No rule has matched yet".to_string()))
        }
    }
}