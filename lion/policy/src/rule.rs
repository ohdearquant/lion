//! Policy rules for enforcement.
//!
//! This module defines the types of rules that can be used
//! to constrain plugin behavior.

use std::path::Path;
use std::sync::Arc;

use lion_core::types::PluginId;

/// Action to take for a policy decision.
#[derive(Debug, Clone)]
pub enum PolicyAction {
    /// Allow the action.
    Allow,
    
    /// Deny the action with a reason.
    Deny(String),
    
    /// Log the action but allow it.
    Log(String),
}

/// Interface for policy rules.
pub trait PolicyRule: Send + Sync {
    /// Get the rule ID.
    fn get_id(&self) -> &str;
    
    /// Get the rule description.
    fn get_description(&self) -> &str;
    
    /// Evaluate file access.
    fn evaluate_file_access(
        &self,
        plugin_id: &PluginId,
        path: &Path,
        write: bool,
    ) -> Option<PolicyAction>;
    
    /// Evaluate network access.
    fn evaluate_network_access(
        &self,
        plugin_id: &PluginId,
        host: &str,
        port: u16,
        listen: bool,
    ) -> Option<PolicyAction>;
    
    /// Evaluate resource usage.
    fn evaluate_resource_usage(
        &self,
        plugin_id: &PluginId,
        resource_type: &str,
        amount: u64,
    ) -> Option<PolicyAction>;
}

/// Rule for file access.
pub struct FileRule {
    /// Rule ID.
    id: String,
    
    /// Rule description.
    description: String,
    
    /// Plugin ID pattern to match.
    plugin_pattern: Option<regex::Regex>,
    
    /// Path pattern to match.
    path_pattern: regex::Regex,
    
    /// Whether this rule applies to write operations.
    write: bool,
    
    /// Whether this rule applies to read operations.
    read: bool,
    
    /// Action to take.
    action: PolicyAction,
}

impl FileRule {
    /// Create a new file rule.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        path_pattern: impl AsRef<str>,
        read: bool,
        write: bool,
        action: PolicyAction,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            id: id.into(),
            description: description.into(),
            plugin_pattern: None,
            path_pattern: regex::Regex::new(path_pattern.as_ref())?,
            write,
            read,
            action,
        })
    }
    
    /// Add a plugin pattern.
    pub fn with_plugin_pattern(mut self, pattern: impl AsRef<str>) -> Result<Self, regex::Error> {
        self.plugin_pattern = Some(regex::Regex::new(pattern.as_ref())?);
        Ok(self)
    }
}

impl PolicyRule for FileRule {
    fn get_id(&self) -> &str {
        &self.id
    }
    
    fn get_description(&self) -> &str {
        &self.description
    }
    
    fn evaluate_file_access(
        &self,
        plugin_id: &PluginId,
        path: &Path,
        write: bool,
    ) -> Option<PolicyAction> {
        // Check plugin pattern
        if let Some(ref pattern) = self.plugin_pattern {
            if !pattern.is_match(&plugin_id.to_string()) {
                return None;
            }
        }
        
        // Check operation type
        if write && !self.write || !write && !self.read {
            return None;
        }
        
        // Check path pattern
        let path_str = path.to_string_lossy();
        if !self.path_pattern.is_match(&path_str) {
            return None;
        }
        
        // Rule matched
        Some(self.action.clone())
    }
    
    fn evaluate_network_access(
        &self,
        _plugin_id: &PluginId,
        _host: &str,
        _port: u16,
        _listen: bool,
    ) -> Option<PolicyAction> {
        // This rule doesn't apply to network access
        None
    }
    
    fn evaluate_resource_usage(
        &self,
        _plugin_id: &PluginId,
        _resource_type: &str,
        _amount: u64,
    ) -> Option<PolicyAction> {
        // This rule doesn't apply to resource usage
        None
    }
}

/// Rule for network access.
pub struct NetworkRule {
    /// Rule ID.
    id: String,
    
    /// Rule description.
    description: String,
    
    /// Plugin ID pattern to match.
    plugin_pattern: Option<regex::Regex>,
    
    /// Host pattern to match.
    host_pattern: regex::Regex,
    
    /// Port range to match.
    port_range: Option<(u16, u16)>,
    
    /// Whether this rule applies to outbound connections.
    connect: bool,
    
    /// Whether this rule applies to inbound connections.
    listen: bool,
    
    /// Action to take.
    action: PolicyAction,
}

impl NetworkRule {
    /// Create a new network rule.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        host_pattern: impl AsRef<str>,
        connect: bool,
        listen: bool,
        action: PolicyAction,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            id: id.into(),
            description: description.into(),
            plugin_pattern: None,
            host_pattern: regex::Regex::new(host_pattern.as_ref())?,
            port_range: None,
            connect,
            listen,
            action,
        })
    }
    
    /// Add a plugin pattern.
    pub fn with_plugin_pattern(mut self, pattern: impl AsRef<str>) -> Result<Self, regex::Error> {
        self.plugin_pattern = Some(regex::Regex::new(pattern.as_ref())?);
        Ok(self)
    }
    
    /// Add a port range.
    pub fn with_port_range(mut self, min: u16, max: u16) -> Self {
        self.port_range = Some((min, max));
        self
    }
}

impl PolicyRule for NetworkRule {
    fn get_id(&self) -> &str {
        &self.id
    }
    
    fn get_description(&self) -> &str {
        &self.description
    }
    
    fn evaluate_file_access(
        &self,
        _plugin_id: &PluginId,
        _path: &Path,
        _write: bool,
    ) -> Option<PolicyAction> {
        // This rule doesn't apply to file access
        None
    }
    
    fn evaluate_network_access(
        &self,
        plugin_id: &PluginId,
        host: &str,
        port: u16,
        listen: bool,
    ) -> Option<PolicyAction> {
        // Check plugin pattern
        if let Some(ref pattern) = self.plugin_pattern {
            if !pattern.is_match(&plugin_id.to_string()) {
                return None;
            }
        }
        
        // Check operation type
        if listen && !self.listen || !listen && !self.connect {
            return None;
        }
        
        // Check host pattern
        if !self.host_pattern.is_match(host) {
            return None;
        }
        
        // Check port range
        if let Some((min, max)) = self.port_range {
            if port < min || port > max {
                return None;
            }
        }
        
        // Rule matched
        Some(self.action.clone())
    }
    
    fn evaluate_resource_usage(
        &self,
        _plugin_id: &PluginId,
        _resource_type: &str,
        _amount: u64,
    ) -> Option<PolicyAction> {
        // This rule doesn't apply to resource usage
        None
    }
}

/// Rule for resource usage limits.
pub struct ResourceLimitRule {
    /// Rule ID.
    id: String,
    
    /// Rule description.
    description: String,
    
    /// Plugin ID pattern to match.
    plugin_pattern: Option<regex::Regex>,
    
    /// Resource type to match.
    resource_type: String,
    
    /// Maximum allowed amount.
    max_amount: u64,
    
    /// Action to take.
    action: PolicyAction,
}

impl ResourceLimitRule {
    /// Create a new resource limit rule.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        resource_type: impl Into<String>,
        max_amount: u64,
        action: PolicyAction,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            plugin_pattern: None,
            resource_type: resource_type.into(),
            max_amount,
            action,
        }
    }
    
    /// Add a plugin pattern.
    pub fn with_plugin_pattern(mut self, pattern: impl AsRef<str>) -> Result<Self, regex::Error> {
        self.plugin_pattern = Some(regex::Regex::new(pattern.as_ref())?);
        Ok(self)
    }
}

impl PolicyRule for ResourceLimitRule {
    fn get_id(&self) -> &str {
        &self.id
    }
    
    fn get_description(&self) -> &str {
        &self.description
    }
    
    fn evaluate_file_access(
        &self,
        _plugin_id: &PluginId,
        _path: &Path,
        _write: bool,
    ) -> Option<PolicyAction> {
        // This rule doesn't apply to file access
        None
    }
    
    fn evaluate_network_access(
        &self,
        _plugin_id: &PluginId,
        _host: &str,
        _port: u16,
        _listen: bool,
    ) -> Option<PolicyAction> {
        // This rule doesn't apply to network access
        None
    }
    
    fn evaluate_resource_usage(
        &self,
        plugin_id: &PluginId,
        resource_type: &str,
        amount: u64,
    ) -> Option<PolicyAction> {
        // Check plugin pattern
        if let Some(ref pattern) = self.plugin_pattern {
            if !pattern.is_match(&plugin_id.to_string()) {
                return None;
            }
        }
        
        // Check resource type
        if resource_type != self.resource_type {
            return None;
        }
        
        // Check amount
        if amount <= self.max_amount {
            // Within limit, allow
            Some(PolicyAction::Allow)
        } else {
            // Exceeds limit, apply action
            Some(self.action.clone())
        }
    }
}