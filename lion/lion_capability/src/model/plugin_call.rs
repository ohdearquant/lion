//! Plugin call capability model.
//! 
//! This module defines capabilities for calling plugins.

use std::collections::HashMap;
use lion_core::error::{Result, CapabilityError};
use lion_core::types::AccessRequest;

use super::capability::{Capability, Constraint};

/// A capability that grants permission to call plugin functions.
#[derive(Debug, Clone)]
pub struct PluginCallCapability {
    /// The plugin-function pairs that are allowed.
    allowed_calls: HashMap<String, Vec<String>>,
}

impl PluginCallCapability {
    /// Create a new plugin call capability.
    ///
    /// # Arguments
    ///
    /// * `allowed_calls` - Map of plugin IDs to allowed functions.
    ///
    /// # Returns
    ///
    /// A new plugin call capability.
    pub fn new(allowed_calls: HashMap<String, Vec<String>>) -> Self {
        Self { allowed_calls }
    }
    
    /// Create a new plugin call capability allowing specific functions.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `functions` - The functions that are allowed.
    ///
    /// # Returns
    ///
    /// A new plugin call capability.
    pub fn for_plugin(plugin_id: impl Into<String>, functions: impl IntoIterator<Item = String>) -> Self {
        let mut allowed_calls = HashMap::new();
        allowed_calls.insert(plugin_id.into(), functions.into_iter().collect());
        Self { allowed_calls }
    }
    
    /// Check if a call is allowed.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `function` - The function name.
    ///
    /// # Returns
    ///
    /// `true` if the call is allowed, `false` otherwise.
    fn is_call_allowed(&self, plugin_id: &str, function: &str) -> bool {
        match self.allowed_calls.get(plugin_id) {
            Some(functions) => functions.contains(&function.to_string()),
            None => false,
        }
    }
    
    /// Get the allowed calls.
    pub fn allowed_calls(&self) -> &HashMap<String, Vec<String>> {
        &self.allowed_calls
    }
}

impl Capability for PluginCallCapability {
    fn capability_type(&self) -> &str {
        "plugin_call"
    }
    
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::PluginCall { plugin_id, function } => {
                // Check if the call is allowed
                if !self.is_call_allowed(plugin_id, function) {
                    return Err(CapabilityError::PermissionDenied(
                        format!("Call to function {} in plugin {} is not allowed", function, plugin_id)
                    ).into());
                }
                
                Ok(())
            },
            _ => Err(CapabilityError::PermissionDenied(
                "Only plugin calls are allowed".into()
            ).into()),
        }
    }
    
    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut allowed_calls = self.allowed_calls.clone();
        
        for constraint in constraints {
            match constraint {
                Constraint::PluginCall { plugin_id, function } => {
                    // Check if the plugin is already allowed
                    if let Some(functions) = allowed_calls.get_mut(plugin_id) {
                        // Check if the function is already allowed
                        if !functions.contains(function) {
                            functions.push(function.clone());
                        }
                    } else {
                        // Add the plugin and function
                        allowed_calls.insert(plugin_id.clone(), vec![function.clone()]);
                    }
                },
                _ => return Err(CapabilityError::ConstraintError(
                    format!("Constraint type {} not supported for plugin call capability", constraint.constraint_type())
                ).into()),
            }
        }
        
        Ok(Box::new(Self { allowed_calls }))
    }
    
    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut capabilities = Vec::new();
        
        // Split by plugin
        for (plugin_id, functions) in &self.allowed_calls {
            let mut allowed_calls = HashMap::new();
            allowed_calls.insert(plugin_id.clone(), functions.clone());
            capabilities.push(Box::new(Self { allowed_calls }) as Box<dyn Capability>);
        }
        
        // If we didn't split, just clone
        if capabilities.is_empty() {
            capabilities.push(Box::new(self.clone()));
        }
        
        capabilities
    }
    
    fn can_join_with(&self, other: &dyn Capability) -> bool {
        other.capability_type() == "plugin_call"
    }
    
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        if !self.can_join_with(other) {
            return Err(CapabilityError::CompositionError(
                format!("Cannot join plugin call capability with {}", other.capability_type())
            ).into());
        }
        
        // Try to get more precise information by checking specific calls
        let mut allowed_calls = self.allowed_calls.clone();
        
        // For each plugin-function pair
        for (plugin_id, functions) in &self.allowed_calls {
            for function in functions {
                // Check if the other capability permits this call
                if other.permits(&AccessRequest::PluginCall {
                    plugin_id: plugin_id.clone(),
                    function: function.clone(),
                }).is_ok() {
                    // The other capability also allows this call
                    // It would be in the joined capability anyway
                }
            }
        }
        
        // TODO: More precise information from the other capability
        
        // Just merge the allowed calls
        let other_calls = match other.permits(&AccessRequest::PluginCall {
            plugin_id: "test".to_string(),
            function: "test".to_string(),
        }) {
            Ok(()) => {
                // If it permits everything, it's probably a super-capability
                HashMap::new()
            },
            Err(_) => {
                // Just add the other's allowed calls
                HashMap::new() // Placeholder
            }
        };
        
        for (plugin_id, functions) in other_calls {
            if let Some(existing_functions) = allowed_calls.get_mut(&plugin_id) {
                // Merge the functions
                for function in functions {
                    if !existing_functions.contains(&function) {
                        existing_functions.push(function);
                    }
                }
            } else {
                // Add the plugin and functions
                allowed_calls.insert(plugin_id, functions);
            }
        }
        
        Ok(Box::new(Self { allowed_calls }))
    }
    
    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_call_capability_permits() {
        let mut allowed_calls = HashMap::new();
        allowed_calls.insert("plugin1".to_string(), vec!["function1".to_string(), "function2".to_string()]);
        let capability = PluginCallCapability::new(allowed_calls);
        
        // Test allowed call
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function1".to_string(),
        };
        assert!(capability.permits(&request).is_ok());
        
        // Test disallowed call
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function3".to_string(),
        };
        assert!(capability.permits(&request).is_err());
        
        // Test call to disallowed plugin
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin2".to_string(),
            function: "function1".to_string(),
        };
        assert!(capability.permits(&request).is_err());
        
        // Test non-plugin call access
        let request = AccessRequest::File {
            path: std::path::PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        assert!(capability.permits(&request).is_err());
    }
    
    #[test]
    fn test_plugin_call_capability_constrain() {
        let mut allowed_calls = HashMap::new();
        allowed_calls.insert("plugin1".to_string(), vec!["function1".to_string()]);
        let capability = PluginCallCapability::new(allowed_calls);
        
        // Constrain to add a new function
        let constraints = vec![Constraint::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function2".to_string(),
        }];
        let constrained = capability.constrain(&constraints).unwrap();
        
        // Should allow call to the original function
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function1".to_string(),
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should allow call to the new function
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function2".to_string(),
        };
        assert!(constrained.permits(&request).is_ok());
        
        // Should still deny call to other functions
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function3".to_string(),
        };
        assert!(constrained.permits(&request).is_err());
    }
    
    #[test]
    fn test_plugin_call_capability_split() {
        let mut allowed_calls = HashMap::new();
        allowed_calls.insert("plugin1".to_string(), vec!["function1".to_string()]);
        allowed_calls.insert("plugin2".to_string(), vec!["function2".to_string()]);
        let capability = PluginCallCapability::new(allowed_calls);
        
        let split = capability.split();
        assert_eq!(split.len(), 2);
        
        // Check that the first capability allows plugin1 but not plugin2
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function1".to_string(),
        };
        let allows_plugin1 = split[0].permits(&request).is_ok() || split[1].permits(&request).is_ok();
        assert!(allows_plugin1);
        
        // Check that the second capability allows plugin2 but not plugin1
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin2".to_string(),
            function: "function2".to_string(),
        };
        let allows_plugin2 = split[0].permits(&request).is_ok() || split[1].permits(&request).is_ok();
        assert!(allows_plugin2);
    }
    
    #[test]
    fn test_plugin_call_capability_join() {
        let mut allowed_calls1 = HashMap::new();
        allowed_calls1.insert("plugin1".to_string(), vec!["function1".to_string()]);
        let capability1 = PluginCallCapability::new(allowed_calls1);
        
        let mut allowed_calls2 = HashMap::new();
        allowed_calls2.insert("plugin2".to_string(), vec!["function2".to_string()]);
        let capability2 = PluginCallCapability::new(allowed_calls2);
        
        let joined = capability1.join(&capability2).unwrap();
        
        // Should allow call to plugin1.function1
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function1".to_string(),
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should allow call to plugin2.function2
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin2".to_string(),
            function: "function2".to_string(),
        };
        assert!(joined.permits(&request).is_ok());
        
        // Should deny call to plugin1.function2
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function2".to_string(),
        };
        assert!(joined.permits(&request).is_err());
        
        // Should deny call to plugin2.function1
        let request = AccessRequest::PluginCall {
            plugin_id: "plugin2".to_string(),
            function: "function1".to_string(),
        };
        assert!(joined.permits(&request).is_err());
    }
}