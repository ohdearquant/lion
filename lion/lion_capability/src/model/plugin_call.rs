use std::any::Any;
use std::collections::{HashMap, HashSet};

use lion_core::id::PluginId;

use super::capability::{AccessRequest, Capability, CapabilityError, Constraint};

/// Represents a capability to call functions in other plugins
#[derive(Debug, Clone)]
pub struct PluginCallCapability {
    /// Map of plugin IDs to sets of allowed function names
    plugin_functions: HashMap<PluginId, HashSet<String>>,

    /// If true, allow all functions for the specified plugins
    /// If a plugin is in the map, this is checked after the specific functions
    allow_all_functions: HashMap<PluginId, bool>,
}

impl PluginCallCapability {
    /// Creates a new plugin call capability
    pub fn new() -> Self {
        Self {
            plugin_functions: HashMap::new(),
            allow_all_functions: HashMap::new(),
        }
    }

    /// Creates a new plugin call capability with a specific plugin's functions
    pub fn with_plugin(plugin_id: PluginId, functions: HashSet<String>) -> Self {
        let mut plugin_functions = HashMap::new();
        plugin_functions.insert(plugin_id, functions);

        let mut allow_all_functions = HashMap::new();
        allow_all_functions.insert(plugin_id, false);

        Self {
            plugin_functions,
            allow_all_functions,
        }
    }

    /// Creates a new plugin call capability allowing all functions in a specific plugin
    pub fn all_functions(plugin_id: PluginId) -> Self {
        let mut plugin_functions = HashMap::new();
        plugin_functions.insert(plugin_id, HashSet::new());

        let mut allow_all_functions = HashMap::new();
        allow_all_functions.insert(plugin_id, true);

        Self {
            plugin_functions,
            allow_all_functions,
        }
    }

    /// Add a function to the allowed set for a plugin
    pub fn add_function(&mut self, plugin_id: PluginId, function: String) {
        self.plugin_functions
            .entry(plugin_id)
            .or_insert_with(HashSet::new)
            .insert(function);

        // Make sure the plugin is in the allow_all_functions map
        self.allow_all_functions.entry(plugin_id).or_insert(false);
    }

    /// Set whether all functions are allowed for a plugin
    pub fn set_allow_all_functions(&mut self, plugin_id: PluginId, allow: bool) {
        self.allow_all_functions.insert(plugin_id, allow);

        // Make sure the plugin is in the plugin_functions map
        self.plugin_functions
            .entry(plugin_id)
            .or_insert_with(HashSet::new);
    }

    /// Get the set of allowed functions for a plugin
    pub fn allowed_functions(&self, plugin_id: &PluginId) -> Option<&HashSet<String>> {
        self.plugin_functions.get(plugin_id)
    }

    /// Returns true if all functions are allowed for a plugin
    pub fn are_all_functions_allowed(&self, plugin_id: &PluginId) -> bool {
        self.allow_all_functions
            .get(plugin_id)
            .copied()
            .unwrap_or(false)
    }

    /// Checks if a function call to a plugin is allowed
    pub fn is_call_allowed(&self, plugin_id: &PluginId, function: &str) -> bool {
        // Check if we have an entry for this plugin
        if let Some(allowed_functions) = self.plugin_functions.get(plugin_id) {
            // Check if all functions are allowed
            if *self.allow_all_functions.get(plugin_id).unwrap_or(&false) {
                return true;
            }

            // Check if the specific function is allowed
            return allowed_functions.contains(function);
        }

        false
    }

    /// Applies a plugin call constraint to this capability
    fn apply_plugin_call_constraint(
        &self,
        plugin_id: &str,
        function: &str,
    ) -> Result<Self, CapabilityError> {
        // Parse the plugin ID
        let plugin_id = match plugin_id.parse::<PluginId>() {
            Ok(id) => id,
            Err(_) => {
                return Err(CapabilityError::InvalidConstraint(format!(
                    "Invalid plugin ID: {}",
                    plugin_id
                )))
            }
        };

        // Check if the call is allowed
        if !self.is_call_allowed(&plugin_id, function) {
            return Err(CapabilityError::InvalidConstraint(format!(
                "Function '{}' in plugin '{}' is not allowed by this capability",
                function, plugin_id
            )));
        }

        // Create a new capability with just this function call
        let mut plugin_functions = HashMap::new();
        let mut functions = HashSet::new();
        functions.insert(function.to_string());
        plugin_functions.insert(plugin_id, functions);

        let mut allow_all_functions = HashMap::new();
        allow_all_functions.insert(plugin_id, false);

        Ok(Self {
            plugin_functions,
            allow_all_functions,
        })
    }
}

impl Capability for PluginCallCapability {
    fn capability_type(&self) -> &str {
        "plugin_call"
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::PluginCall {
                plugin_id,
                function,
            } => {
                // Parse the plugin ID
                let plugin_id = match plugin_id.parse::<PluginId>() {
                    Ok(id) => id,
                    Err(_) => {
                        return Err(CapabilityError::AccessDenied(format!(
                            "Invalid plugin ID: {}",
                            plugin_id
                        )))
                    }
                };

                // Check if the call is allowed
                if self.is_call_allowed(&plugin_id, function) {
                    Ok(())
                } else {
                    Err(CapabilityError::AccessDenied(format!(
                        "Function '{}' in plugin '{}' is not allowed by this capability",
                        function, plugin_id
                    )))
                }
            }
            _ => Err(CapabilityError::IncompatibleTypes(format!(
                "Expected PluginCall request, got {:?}",
                request
            ))),
        }
    }

    fn constrain(
        &self,
        constraints: &[Constraint],
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut result = self.clone();

        for constraint in constraints {
            match constraint {
                Constraint::PluginCall {
                    plugin_id,
                    function,
                } => {
                    result = result.apply_plugin_call_constraint(plugin_id, function)?;
                }
                _ => {
                    return Err(CapabilityError::InvalidConstraint(format!(
                        "Constraint {:?} not applicable to PluginCallCapability",
                        constraint
                    )))
                }
            }
        }

        Ok(Box::new(result))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut result = Vec::new();

        // Split by plugin
        for (plugin_id, functions) in &self.plugin_functions {
            if *self.allow_all_functions.get(plugin_id).unwrap_or(&false) {
                // For plugins where all functions are allowed, create one capability
                result.push(Box::new(PluginCallCapability::all_functions(*plugin_id))
                    as Box<dyn Capability>);
            } else {
                // For plugins with specific functions, split by function
                for function in functions {
                    let mut cap = PluginCallCapability::new();
                    cap.add_function(*plugin_id, function.clone());
                    result.push(Box::new(cap) as Box<dyn Capability>);
                }
            }
        }

        result
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to PluginCallCapability
        if let Some(other_call) = other.as_any().downcast_ref::<PluginCallCapability>() {
            let mut result = self.clone();

            // Merge plugin functions
            for (plugin_id, functions) in &other_call.plugin_functions {
                let entry = result
                    .plugin_functions
                    .entry(*plugin_id)
                    .or_insert_with(HashSet::new);
                entry.extend(functions.iter().cloned());
            }

            // Merge allow_all_functions
            for (plugin_id, allow) in &other_call.allow_all_functions {
                let current = result
                    .allow_all_functions
                    .entry(*plugin_id)
                    .or_insert(false);
                *current = *current || *allow;
            }

            Ok(Box::new(result))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot join PluginCallCapability with a different capability type".to_string(),
            ))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to PluginCallCapability
        if let Some(other_call) = other.as_any().downcast_ref::<PluginCallCapability>() {
            // This capability is <= other if for each plugin:
            // 1. If self allows all functions, other must also allow all functions
            // 2. All functions allowed by self must be allowed by other

            for (plugin_id, _) in &self.plugin_functions {
                // Check if other has this plugin
                if !other_call.plugin_functions.contains_key(plugin_id) {
                    return false;
                }

                // If self allows all functions, other must too
                if *self.allow_all_functions.get(plugin_id).unwrap_or(&false) {
                    if !*other_call
                        .allow_all_functions
                        .get(plugin_id)
                        .unwrap_or(&false)
                    {
                        return false;
                    }
                    continue;
                }

                // Check if other allows all functions
                if *other_call
                    .allow_all_functions
                    .get(plugin_id)
                    .unwrap_or(&false)
                {
                    continue;
                }

                // Check if all functions allowed by self are allowed by other
                let self_functions = self.plugin_functions.get(plugin_id).unwrap();
                let other_functions = other_call.plugin_functions.get(plugin_id).unwrap();

                for function in self_functions {
                    if !other_functions.contains(function) {
                        return false;
                    }
                }
            }

            true
        } else {
            false
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to PluginCallCapability
        if let Some(other_call) = other.as_any().downcast_ref::<PluginCallCapability>() {
            let mut result = PluginCallCapability::new();

            // Find intersection of plugins
            for plugin_id in self.plugin_functions.keys() {
                if !other_call.plugin_functions.contains_key(plugin_id) {
                    continue;
                }

                // If both allow all functions, the intersection allows all functions
                let self_allow_all = *self.allow_all_functions.get(plugin_id).unwrap_or(&false);
                let other_allow_all = *other_call
                    .allow_all_functions
                    .get(plugin_id)
                    .unwrap_or(&false);

                if self_allow_all && other_allow_all {
                    result.set_allow_all_functions(*plugin_id, true);
                    continue;
                }

                // If one allows all functions, use the specific functions from the other
                if self_allow_all {
                    let other_functions = other_call.plugin_functions.get(plugin_id).unwrap();
                    for function in other_functions {
                        result.add_function(*plugin_id, function.clone());
                    }
                    continue;
                }

                if other_allow_all {
                    let self_functions = self.plugin_functions.get(plugin_id).unwrap();
                    for function in self_functions {
                        result.add_function(*plugin_id, function.clone());
                    }
                    continue;
                }

                // Otherwise, find intersection of functions
                let self_functions = self.plugin_functions.get(plugin_id).unwrap();
                let other_functions = other_call.plugin_functions.get(plugin_id).unwrap();

                for function in self_functions {
                    if other_functions.contains(function) {
                        result.add_function(*plugin_id, function.clone());
                    }
                }
            }

            // Ensure there are some functions allowed
            if result.plugin_functions.is_empty() {
                return Err(CapabilityError::InvalidState(
                    "No functions in common between the capabilities".to_string(),
                ));
            }

            Ok(Box::new(result))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot compute meet with different capability types".to_string(),
            ))
        }
    }

    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_call_capability_is_call_allowed() {
        let mut cap = PluginCallCapability::new();

        let plugin1 = PluginId::from_u64(1);
        let plugin2 = PluginId::from_u64(2);
        let plugin3 = PluginId::from_u64(3);

        // Add specific functions for plugin1
        cap.add_function(plugin1, "func1".to_string());
        cap.add_function(plugin1, "func2".to_string());

        // Allow all functions for plugin2
        cap.set_allow_all_functions(plugin2, true);

        // Test allowed calls
        assert!(cap.is_call_allowed(&plugin1, "func1"));
        assert!(cap.is_call_allowed(&plugin1, "func2"));
        assert!(!cap.is_call_allowed(&plugin1, "func3"));

        assert!(cap.is_call_allowed(&plugin2, "any_function"));

        // Test disallowed calls
        assert!(!cap.is_call_allowed(&plugin3, "func1"));
    }

    #[test]
    fn test_plugin_call_capability_permits() {
        let mut cap = PluginCallCapability::new();

        let plugin1 = PluginId::from_u64(1);

        // Add specific functions for plugin1
        cap.add_function(plugin1, "func1".to_string());

        // Valid function call
        assert!(cap
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func1".to_string(),
            })
            .is_ok());

        // Invalid function
        assert!(cap
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func2".to_string(),
            })
            .is_err());

        // Invalid plugin
        assert!(cap
            .permits(&AccessRequest::PluginCall {
                plugin_id: PluginId::from_u64(2).to_string(),
                function: "func1".to_string(),
            })
            .is_err());

        // Invalid request type
        assert!(cap
            .permits(&AccessRequest::File {
                path: "/tmp/file.txt".to_string(),
                read: true,
                write: false,
                execute: false,
            })
            .is_err());
    }

    #[test]
    fn test_plugin_call_capability_constrain() {
        let mut cap = PluginCallCapability::new();

        let plugin1 = PluginId::from_u64(1);

        // Add several functions for plugin1
        cap.add_function(plugin1, "func1".to_string());
        cap.add_function(plugin1, "func2".to_string());

        // Constrain to just one function
        let constrained = cap
            .constrain(&[Constraint::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func1".to_string(),
            }])
            .unwrap();

        // The constrained capability should allow just this function
        assert!(constrained
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func1".to_string(),
            })
            .is_ok());

        assert!(constrained
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func2".to_string(),
            })
            .is_err());
    }

    #[test]
    fn test_plugin_call_capability_leq() {
        let mut cap1 = PluginCallCapability::new();
        let mut cap2 = PluginCallCapability::new();

        let plugin1 = PluginId::from_u64(1);

        // Add specific function to cap1
        cap1.add_function(plugin1, "func1".to_string());

        // Add more functions to cap2
        cap2.add_function(plugin1, "func1".to_string());
        cap2.add_function(plugin1, "func2".to_string());

        // cap1 <= cap2 because cap2 allows all functions that cap1 allows
        assert!(cap1.leq(&cap2));

        // cap2 !<= cap1 because cap2 allows func2 which cap1 doesn't
        assert!(!cap2.leq(&cap1));

        // Test with allow_all_functions
        let mut cap3 = PluginCallCapability::new();
        cap3.set_allow_all_functions(plugin1, true);

        // cap1 <= cap3 because cap3 allows all functions
        assert!(cap1.leq(&cap3));

        // cap3 !<= cap1 because cap3 allows all functions, but cap1 doesn't
        assert!(!cap3.leq(&cap1));
    }

    #[test]
    fn test_plugin_call_capability_join_and_meet() {
        let mut cap1 = PluginCallCapability::new();
        let mut cap2 = PluginCallCapability::new();

        let plugin1 = PluginId::from_u64(1);

        // Add different functions to each capability
        cap1.add_function(plugin1, "func1".to_string());
        cap1.add_function(plugin1, "func2".to_string());

        cap2.add_function(plugin1, "func2".to_string());
        cap2.add_function(plugin1, "func3".to_string());

        // Join
        let join = cap1.join(&cap2).unwrap();

        // The joined capability should allow all functions
        assert!(join
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func1".to_string(),
            })
            .is_ok());

        assert!(join
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func2".to_string(),
            })
            .is_ok());

        assert!(join
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func3".to_string(),
            })
            .is_ok());

        // Meet
        let meet = cap1.meet(&cap2).unwrap();

        // The meet capability should only allow func2 which is common to both
        assert!(meet
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func2".to_string(),
            })
            .is_ok());

        assert!(meet
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func1".to_string(),
            })
            .is_err());

        assert!(meet
            .permits(&AccessRequest::PluginCall {
                plugin_id: plugin1.to_string(),
                function: "func3".to_string(),
            })
            .is_err());
    }
}
