//! Policy aggregation.
//! 
//! This module provides functionality for aggregating policies.

use std::collections::HashMap;
use lion_core::error::Result;
use lion_core::id::PluginId;

use crate::model::{PolicyRule, PolicyAction, PolicyObject, PolicySubject};
use crate::store::PolicyStore;

/// A policy aggregator.
///
/// This aggregator provides functionality for aggregating policies.
pub struct PolicyAggregator<P> {
    /// The policy store.
    policy_store: P,
}

impl<P> PolicyAggregator<P>
where
    P: PolicyStore,
{
    /// Create a new policy aggregator.
    ///
    /// # Arguments
    ///
    /// * `policy_store` - The policy store.
    ///
    /// # Returns
    ///
    /// A new policy aggregator.
    pub fn new(policy_store: P) -> Self {
        Self { policy_store }
    }
    
    /// Get all policies for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get policies for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PolicyRule>)` - The policies.
    /// * `Err` - If the policies could not be retrieved.
    pub fn get_policies_for_plugin(&self, plugin_id: &PluginId) -> Result<Vec<PolicyRule>> {
        self.policy_store.list_rules_matching(|rule| {
            matches!(
                rule.subject,
                PolicySubject::Any |
                PolicySubject::Plugin(ref id) if id == plugin_id |
                PolicySubject::Plugins(ref ids) if ids.contains(plugin_id)
            )
        })
    }
    
    /// Get policies by object type.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get policies for.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, Vec<PolicyRule>>)` - The policies, grouped by object type.
    /// * `Err` - If the policies could not be retrieved.
    pub fn get_policies_by_object_type(&self, plugin_id: &PluginId) -> Result<HashMap<String, Vec<PolicyRule>>> {
        let policies = self.get_policies_for_plugin(plugin_id)?;
        let mut by_type = HashMap::new();
        
        for policy in policies {
            let object_type = match &policy.object {
                PolicyObject::Any => "any".to_string(),
                PolicyObject::File(_) => "file".to_string(),
                PolicyObject::Network(_) => "network".to_string(),
                PolicyObject::PluginCall(_) => "plugin_call".to_string(),
                PolicyObject::Memory(_) => "memory".to_string(),
                PolicyObject::Message(_) => "message".to_string(),
                PolicyObject::Custom { object_type, .. } => object_type.clone(),
            };
            
            by_type.entry(object_type).or_insert_with(Vec::new).push(policy);
        }
        
        Ok(by_type)
    }
    
    /// Get policies by action.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get policies for.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, Vec<PolicyRule>>)` - The policies, grouped by action.
    /// * `Err` - If the policies could not be retrieved.
    pub fn get_policies_by_action(&self, plugin_id: &PluginId) -> Result<HashMap<String, Vec<PolicyRule>>> {
        let policies = self.get_policies_for_plugin(plugin_id)?;
        let mut by_action = HashMap::new();
        
        for policy in policies {
            let action_type = match &policy.action {
                PolicyAction::Allow => "allow".to_string(),
                PolicyAction::Deny => "deny".to_string(),
                PolicyAction::AllowWithConstraints(_) => "allow_with_constraints".to_string(),
                PolicyAction::TransformToConstraints(_) => "transform_to_constraints".to_string(),
                PolicyAction::Audit => "audit".to_string(),
            };
            
            by_action.entry(action_type).or_insert_with(Vec::new).push(policy);
        }
        
        Ok(by_action)
    }
    
    /// Merge policies with the same object and action.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PolicyRule>)` - The merged policies.
    /// * `Err` - If the policies could not be merged.
    pub fn merge_policies(&self) -> Result<Vec<PolicyRule>> {
        let policies = self.policy_store.list_rules()?;
        let mut by_key = HashMap::new();
        
        // Group policies by subject, object, and action
        for policy in policies {
            let key = (policy.subject.clone(), policy.object.clone(), policy.action.clone());
            by_key.entry(key).or_insert_with(Vec::new).push(policy);
        }
        
        // Merge policies with the same key
        let mut merged = Vec::new();
        
        for (_, policies) in by_key {
            if policies.len() == 1 {
                merged.push(policies[0].clone());
                continue;
            }
            
            // Get the policy with the highest priority
            let mut highest_priority = &policies[0];
            
            for policy in &policies {
                if policy.priority > highest_priority.priority {
                    highest_priority = policy;
                }
            }
            
            merged.push(highest_priority.clone());
        }
        
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::InMemoryPolicyStore;
    use crate::model::{PolicySubject, PolicyObject, PolicyAction, FileObject, NetworkObject};
    
    #[test]
    fn test_get_policies_for_plugin() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let aggregator = PolicyAggregator::new(policy_store.clone());
        
        // Create plugins
        let plugin1 = PluginId::new();
        let plugin2 = PluginId::new();
        
        // Create policy rules
        let rule1 = PolicyRule::new(
            "rule1",
            "Plugin 1 Rule",
            "A rule for plugin 1",
            PolicySubject::Plugin(plugin1.clone()),
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        let rule2 = PolicyRule::new(
            "rule2",
            "Plugin 2 Rule",
            "A rule for plugin 2",
            PolicySubject::Plugin(plugin2.clone()),
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        let rule3 = PolicyRule::new(
            "rule3",
            "Both Plugins Rule",
            "A rule for both plugins",
            PolicySubject::Plugins(vec![plugin1.clone(), plugin2.clone()]),
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();
        policy_store.add_rule(rule3).unwrap();
        
        // Get policies for plugin 1
        let policies = aggregator.get_policies_for_plugin(&plugin1).unwrap();
        assert_eq!(policies.len(), 2);
        assert!(policies.iter().any(|r| r.id == "rule1"));
        assert!(policies.iter().any(|r| r.id == "rule3"));
        
        // Get policies for plugin 2
        let policies = aggregator.get_policies_for_plugin(&plugin2).unwrap();
        assert_eq!(policies.len(), 2);
        assert!(policies.iter().any(|r| r.id == "rule2"));
        assert!(policies.iter().any(|r| r.id == "rule3"));
    }
    
    #[test]
    fn test_get_policies_by_object_type() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let aggregator = PolicyAggregator::new(policy_store.clone());
        
        // Create a plugin
        let plugin = PluginId::new();
        
        // Create policy rules
        let rule1 = PolicyRule::new(
            "rule1",
            "File Rule",
            "A rule for files",
            PolicySubject::Plugin(plugin.clone()),
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
            "Network Rule",
            "A rule for network",
            PolicySubject::Plugin(plugin.clone()),
            PolicyObject::Network(NetworkObject {
                host: "example.com".to_string(),
                port: None,
                protocol: None,
            }),
            PolicyAction::Allow,
            None,
            0,
        );
        
        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();
        
        // Get policies by object type
        let by_type = aggregator.get_policies_by_object_type(&plugin).unwrap();
        
        assert_eq!(by_type.len(), 2);
        assert!(by_type.contains_key("file"));
        assert!(by_type.contains_key("network"));
        assert_eq!(by_type.get("file").unwrap().len(), 1);
        assert_eq!(by_type.get("network").unwrap().len(), 1);
    }
    
    #[test]
    fn test_get_policies_by_action() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let aggregator = PolicyAggregator::new(policy_store.clone());
        
        // Create a plugin
        let plugin = PluginId::new();
        
        // Create policy rules
        let rule1 = PolicyRule::new(
            "rule1",
            "Allow Rule",
            "An allow rule",
            PolicySubject::Plugin(plugin.clone()),
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        let rule2 = PolicyRule::new(
            "rule2",
            "Deny Rule",
            "A deny rule",
            PolicySubject::Plugin(plugin.clone()),
            PolicyObject::Any,
            PolicyAction::Deny,
            None,
            0,
        );
        
        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();
        
        // Get policies by action
        let by_action = aggregator.get_policies_by_action(&plugin).unwrap();
        
        assert_eq!(by_action.len(), 2);
        assert!(by_action.contains_key("allow"));
        assert!(by_action.contains_key("deny"));
        assert_eq!(by_action.get("allow").unwrap().len(), 1);
        assert_eq!(by_action.get("deny").unwrap().len(), 1);
    }
    
    #[test]
    fn test_merge_policies() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let aggregator = PolicyAggregator::new(policy_store.clone());
        
        // Create a plugin
        let plugin = PluginId::new();
        
        // Create policy rules with the same subject, object, and action but different priorities
        let rule1 = PolicyRule::new(
            "rule1",
            "Low Priority Rule",
            "A low priority rule",
            PolicySubject::Plugin(plugin.clone()),
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        let rule2 = PolicyRule::new(
            "rule2",
            "High Priority Rule",
            "A high priority rule",
            PolicySubject::Plugin(plugin.clone()),
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            1,
        );
        
        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();
        
        // Merge the policies
        let merged = aggregator.merge_policies().unwrap();
        
        // Check that only the high priority rule remains
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, "rule2");
    }
}