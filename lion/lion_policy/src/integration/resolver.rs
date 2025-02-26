//! Constraint resolution.
//! 
//! This module provides functionality for resolving constraints.

use std::collections::HashMap;
use lion_core::error::{Result, CapabilityError};
use lion_core::id::PluginId;
use lion_core::types::AccessRequest;

use crate::model::{PolicyRule, PolicyAction, Constraint, EvaluationResult};
use crate::store::PolicyStore;

/// A resolver that resolves constraints for access requests.
pub struct ConstraintResolver<P> {
    /// The policy store.
    policy_store: P,
    
    /// Cached constraints by plugin and request type.
    constraint_cache: HashMap<(PluginId, String), Vec<Constraint>>,
}

impl<P> ConstraintResolver<P>
where
    P: PolicyStore,
{
    /// Create a new constraint resolver.
    ///
    /// # Arguments
    ///
    /// * `policy_store` - The policy store.
    ///
    /// # Returns
    ///
    /// A new constraint resolver.
    pub fn new(policy_store: P) -> Self {
        Self {
            policy_store,
            constraint_cache: HashMap::new(),
        }
    }
    
    /// Clear the constraint cache.
    pub fn clear_cache(&mut self) {
        self.constraint_cache.clear();
    }
    
    /// Get constraints for an access request.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin making the request.
    /// * `request` - The access request.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Constraint>)` - The constraints.
    /// * `Err` - If the constraints could not be retrieved.
    pub fn get_constraints(
        &mut self,
        plugin_id: &PluginId,
        request: &AccessRequest,
    ) -> Result<Vec<Constraint>> {
        // Get the request type
        let request_type = match request {
            AccessRequest::File { .. } => "file",
            AccessRequest::Network { .. } => "network",
            AccessRequest::PluginCall { .. } => "plugin_call",
            AccessRequest::Memory { .. } => "memory",
            AccessRequest::Message { .. } => "message",
            AccessRequest::Custom { resource_type, .. } => resource_type,
        };
        
        // Check if we have cached constraints
        let cache_key = (plugin_id.clone(), request_type.to_string());
        
        if let Some(constraints) = self.constraint_cache.get(&cache_key) {
            return Ok(constraints.clone());
        }
        
        // Get policies that apply to this plugin and request type
        let policies = self.policy_store.list_rules_matching(|rule| {
            // Check if the rule applies to this plugin
            let plugin_match = matches!(
                rule.subject,
                crate::model::PolicySubject::Any |
                crate::model::PolicySubject::Plugin(ref id) if id == plugin_id
            );
            
            // Check if the rule applies to this request type
            let request_match = match request {
                AccessRequest::File { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::File(_))
                },
                AccessRequest::Network { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Network(_))
                },
                AccessRequest::PluginCall { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::PluginCall(_))
                },
                AccessRequest::Memory { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Memory(_))
                },
                AccessRequest::Message { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Message(_))
                },
                AccessRequest::Custom { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Custom { .. })
                },
            };
            
            plugin_match && request_match
        })?;
        
        // Sort policies by priority (higher priority first)
        let mut policies = policies;
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Get constraints from policies
        let mut constraints = Vec::new();
        
        for policy in policies {
            match &policy.action {
                PolicyAction::Allow => {},
                PolicyAction::Deny => {},
                PolicyAction::AllowWithConstraints(constraint_strs) => {
                    for constraint_str in constraint_strs {
                        // Parse constraint from string
                        // This is just a placeholder; in a real implementation, we would
                        // have a proper parser for constraint strings
                        let constraint = Constraint::Custom {
                            constraint_type: "from_string".to_string(),
                            value: constraint_str.clone(),
                        };
                        constraints.push(constraint);
                    }
                },
                PolicyAction::TransformToConstraints(constraint_strs) => {
                    for constraint_str in constraint_strs {
                        // Parse constraint from string
                        let constraint = Constraint::Custom {
                            constraint_type: "from_string".to_string(),
                            value: constraint_str.clone(),
                        };
                        constraints.push(constraint);
                    }
                },
                PolicyAction::Audit => {},
            }
        }
        
        // Cache the constraints
        self.constraint_cache.insert(cache_key, constraints.clone());
        
        Ok(constraints)
    }
    
    /// Evaluate an access request against policies.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin making the request.
    /// * `request` - The access request.
    ///
    /// # Returns
    ///
    /// * `Ok(EvaluationResult)` - The result of the evaluation.
    /// * `Err` - If the evaluation could not be performed.
    pub fn evaluate(
        &mut self,
        plugin_id: &PluginId,
        request: &AccessRequest,
    ) -> Result<EvaluationResult> {
        // Get policies that apply to this plugin and request type
        let policies = self.policy_store.list_rules_matching(|rule| {
            // Check if the rule applies to this plugin
            let plugin_match = matches!(
                rule.subject,
                crate::model::PolicySubject::Any |
                crate::model::PolicySubject::Plugin(ref id) if id == plugin_id
            );
            
            // Check if the rule applies to this request type
            let request_match = match request {
                AccessRequest::File { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::File(_))
                },
                AccessRequest::Network { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Network(_))
                },
                AccessRequest::PluginCall { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::PluginCall(_))
                },
                AccessRequest::Memory { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Memory(_))
                },
                AccessRequest::Message { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Message(_))
                },
                AccessRequest::Custom { .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Custom { .. })
                },
            };
            
            plugin_match && request_match
        })?;
        
        // Sort policies by priority (higher priority first)
        let mut policies = policies;
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // If there are no policies, return NoPolicy
        if policies.is_empty() {
            return Ok(EvaluationResult::NoPolicy);
        }
        
        // Check if any policy allows or denies the request
        for policy in &policies {
            match &policy.action {
                PolicyAction::Allow => return Ok(EvaluationResult::Allow),
                PolicyAction::Deny => return Ok(EvaluationResult::Deny),
                PolicyAction::AllowWithConstraints(_) => {
                    // Get constraints from the policy
                    return Ok(EvaluationResult::from(&policy.action));
                },
                PolicyAction::TransformToConstraints(_) => {
                    // Get constraints from the policy
                    return Ok(EvaluationResult::from(&policy.action));
                },
                PolicyAction::Audit => {
                    // Continue checking other policies
                },
            }
        }
        
        // If no policy explicitly allows or denies, default to deny
        Ok(EvaluationResult::Deny)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryPolicyStore;
    use crate::model::{PolicySubject, PolicyObject, PolicyAction, FileObject};
    use std::path::PathBuf;
    
    #[test]
    fn test_evaluate() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let mut resolver = ConstraintResolver::new(policy_store.clone());
        
        // Create a plugin
        let plugin_id = PluginId::new();
        
        // Create policy rules
        let rule1 = PolicyRule::new(
            "rule1",
            "Allow Rule",
            "An allow rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/tmp".to_string(),
                is_directory: true,
            }),
            PolicyAction::Allow,
            None,
            0,
        );
        
        let rule2 = PolicyRule::new(
            "rule2",
            "Deny Rule",
            "A deny rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/etc".to_string(),
                is_directory: true,
            }),
            PolicyAction::Deny,
            None,
            0,
        );
        
        let rule3 = PolicyRule::new(
            "rule3",
            "Constraint Rule",
            "A constraint rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/var".to_string(),
                is_directory: true,
            }),
            PolicyAction::AllowWithConstraints(vec!["file_operation:read=true,write=false,execute=false".to_string()]),
            None,
            0,
        );
        
        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();
        policy_store.add_rule(rule3).unwrap();
        
        // Evaluate a request that should be allowed
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        let result = resolver.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(result, EvaluationResult::Allow));
        
        // Evaluate a request that should be denied
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        let result = resolver.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(result, EvaluationResult::Deny));
        
        // Evaluate a request that should be allowed with constraints
        let request = AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: true,
            write: false,
            execute: false,
        };
        let result = resolver.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(result, EvaluationResult::AllowWithConstraints(_)));
        
        // Evaluate a request with no matching policy
        let request = AccessRequest::File {
            path: PathBuf::from("/usr/bin/ls"),
            read: true,
            write: false,
            execute: false,
        };
        let result = resolver.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(result, EvaluationResult::NoPolicy));
    }
    
    #[test]
    fn test_get_constraints() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let mut resolver = ConstraintResolver::new(policy_store.clone());
        
        // Create a plugin
        let plugin_id = PluginId::new();
        
        // Create a policy rule with constraints
        let rule = PolicyRule::new(
            "rule1",
            "Constraint Rule",
            "A constraint rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/var".to_string(),
                is_directory: true,
            }),
            PolicyAction::AllowWithConstraints(vec!["file_operation:read=true,write=false,execute=false".to_string()]),
            None,
            0,
        );
        
        // Add the rule to the store
        policy_store.add_rule(rule).unwrap();
        
        // Get constraints for a request
        let request = AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: true,
            write: false,
            execute: false,
        };
        let constraints = resolver.get_constraints(&plugin_id, &request).unwrap();
        
        assert_eq!(constraints.len(), 1);
        assert!(matches!(constraints[0], Constraint::Custom { .. }));
    }
    
    #[test]
    fn test_cache() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let mut resolver = ConstraintResolver::new(policy_store.clone());
        
        // Create a plugin
        let plugin_id = PluginId::new();
        
        // Create a policy rule with constraints
        let rule = PolicyRule::new(
            "rule1",
            "Constraint Rule",
            "A constraint rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::File(FileObject {
                path: "/var".to_string(),
                is_directory: true,
            }),
            PolicyAction::AllowWithConstraints(vec!["file_operation:read=true,write=false,execute=false".to_string()]),
            None,
            0,
        );
        
        // Add the rule to the store
        policy_store.add_rule(rule).unwrap();
        
        // Get constraints for a request
        let request = AccessRequest::File {
            path: PathBuf::from("/var/file"),
            read: true,
            write: false,
            execute: false,
        };
        resolver.get_constraints(&plugin_id, &request).unwrap();
        
        // Remove the rule from the store
        policy_store.remove_rule("rule1").unwrap();
        
        // Get constraints again; should still return the cached value
        let constraints = resolver.get_constraints(&plugin_id, &request).unwrap();
        assert_eq!(constraints.len(), 1);
        
        // Clear the cache
        resolver.clear_cache();
        
        // Get constraints again; should return an empty vector
        let constraints = resolver.get_constraints(&plugin_id, &request).unwrap();
        assert_eq!(constraints.len(), 0);
    }
}