//! Policy evaluation engine.
//!
//! This module provides the policy evaluation engine.

use lion_core::error::Result;
use lion_core::id::PluginId;
use lion_core::types::AccessRequest;
use std::collections::HashMap;

use crate::model::{Evaluation, EvaluationResult, PolicyRule};
use crate::store::PolicyStore;

/// Policy evaluation engine.
///
/// This engine evaluates access requests against policies.
pub struct PolicyEvaluator<P> {
    /// The policy store.
    policy_store: P,

    /// Cache of evaluations by plugin and request.
    evaluation_cache: HashMap<(PluginId, AccessRequest), EvaluationResult>,
}

impl<P> PolicyEvaluator<P>
where
    P: PolicyStore,
{
    /// Create a new policy evaluator.
    ///
    /// # Arguments
    ///
    /// * `policy_store` - The policy store.
    ///
    /// # Returns
    ///
    /// A new policy evaluator.
    pub fn new(policy_store: P) -> Self {
        Self {
            policy_store,
            evaluation_cache: HashMap::new(),
        }
    }

    /// Clear the evaluation cache.
    pub fn clear_cache(&mut self) {
        self.evaluation_cache.clear();
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
    /// * `Ok(Evaluation)` - The evaluation.
    /// * `Err` - If the evaluation could not be performed.
    pub fn evaluate(
        &mut self,
        plugin_id: &PluginId,
        request: &AccessRequest,
    ) -> Result<Evaluation> {
        // Check if we have a cached evaluation
        let cache_key = (plugin_id.clone(), request.clone());

        if let Some(result) = self.evaluation_cache.get(&cache_key) {
            return Ok(Evaluation::new(
                plugin_id.clone(),
                request.clone(),
                result.clone(),
                None,
            ));
        }

        // Get relevant policies
        let policies = self.get_relevant_policies(plugin_id, request)?;

        // If there are no policies, return NoPolicy
        if policies.is_empty() {
            let result = EvaluationResult::NoPolicy;
            self.evaluation_cache.insert(cache_key, result.clone());

            return Ok(Evaluation::new(
                plugin_id.clone(),
                request.clone(),
                result,
                None,
            ));
        }

        // Sort policies by priority (higher priority first)
        let mut policies = policies;
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Evaluate policies in order
        for policy in &policies {
            let result = EvaluationResult::from(&policy.action);

            match result {
                EvaluationResult::Allow
                | EvaluationResult::Deny
                | EvaluationResult::AllowWithConstraints(_) => {
                    // Cache the result
                    self.evaluation_cache.insert(cache_key, result.clone());

                    return Ok(Evaluation::new(
                        plugin_id.clone(),
                        request.clone(),
                        result,
                        Some(policy.clone()),
                    ));
                }
                EvaluationResult::Audit => {
                    // Continue evaluating policies
                }
                EvaluationResult::NoPolicy => {
                    // Should not happen
                }
            }
        }

        // If no policy explicitly allows or denies, default to deny
        let result = EvaluationResult::Deny;
        self.evaluation_cache.insert(cache_key, result.clone());

        Ok(Evaluation::new(
            plugin_id.clone(),
            request.clone(),
            result,
            None,
        ))
    }

    /// Get policies relevant to the given plugin and request.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin making the request.
    /// * `request` - The access request.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PolicyRule>)` - The relevant policies.
    /// * `Err` - If the policies could not be retrieved.
    fn get_relevant_policies(
        &self,
        plugin_id: &PluginId,
        request: &AccessRequest,
    ) -> Result<Vec<PolicyRule>> {
        self.policy_store.list_rules_matching(|rule| {
            // Skip expired rules
            if rule.is_expired() {
                return false;
            }

            // Check if the rule applies to this plugin
            let plugin_match = match &rule.subject {
                crate::model::PolicySubject::Any => true,
                crate::model::PolicySubject::Plugin(id) => id == plugin_id,
                crate::model::PolicySubject::Plugins(ids) => ids.contains(plugin_id),
                // For now, these other subject types don't match any plugin
                // In a real implementation, you'd check against plugin metadata
                crate::model::PolicySubject::PluginName(_) => false,
                crate::model::PolicySubject::PluginTag(_) => false,
                crate::model::PolicySubject::PluginRole(_) => false,
            };

            // Check if the rule applies to this request type
            let request_match = match request {
                AccessRequest::File { path, .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::File(ref file) if path.starts_with(&file.path))
                },
                AccessRequest::Network { host, port, .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Network(ref network) if (network.host == "*" || &network.host == host) && (network.port.is_none() || network.port == Some(*port))
                    )
                },
                AccessRequest::PluginCall { plugin_id, function } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::PluginCall(ref call) if call.plugin_id == *plugin_id && (call.function.is_none() || call.function == Some(function.clone())))
                },
                AccessRequest::Memory { region_id, .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Memory(ref memory) if memory.region_id == *region_id
                    )
                },
                AccessRequest::Message { recipient, topic } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    matches!(rule.object, crate::model::PolicyObject::Message(ref message) if message.recipient == *recipient && (message.topic.is_none() || message.topic == Some(topic.clone())))},
                AccessRequest::Custom { resource_type, operation, .. } => {
                    matches!(rule.object, crate::model::PolicyObject::Any) ||
                    if let crate::model::PolicyObject::Custom { ref object_type, ref value } = rule.object {
                        object_type == resource_type && value == operation
                    } else {
                        false
                    }
                },
            };plugin_match && request_match})
    }

    /// Check if a request is allowed.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin making the request.
    /// * `request` - The access request.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - Whether the request is allowed.
    /// * `Err` - If the check could not be performed.
    pub fn is_allowed(&mut self, plugin_id: &PluginId, request: &AccessRequest) -> Result<bool> {
        let evaluation = self.evaluate(plugin_id, request)?;

        match evaluation.result {
            EvaluationResult::Allow => Ok(true),
            EvaluationResult::AllowWithConstraints(_) => Ok(true),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        rule::{FileObject, NetworkObject, PluginCallObject},
        PolicyAction, PolicyObject, PolicySubject,
    };
    use crate::store::InMemoryPolicyStore;
    use std::path::PathBuf;

    #[test]
    fn test_evaluate() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let mut evaluator = PolicyEvaluator::new(policy_store.clone());

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

        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();

        // Evaluate a request that should be allowed
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        let evaluation = evaluator.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(evaluation.result, EvaluationResult::Allow));

        // Evaluate a request that should be denied
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        let evaluation = evaluator.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(evaluation.result, EvaluationResult::Deny));

        // Evaluate a request with no matching policy
        let request = AccessRequest::File {
            path: PathBuf::from("/usr/bin/ls"),
            read: true,
            write: false,
            execute: false,
        };
        let evaluation = evaluator.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(evaluation.result, EvaluationResult::NoPolicy));
    }

    #[test]
    fn test_is_allowed() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let mut evaluator = PolicyEvaluator::new(policy_store.clone());

        // Create a plugin
        let plugin_id = PluginId::new();

        // Create policy rules
        let rule1 = PolicyRule::new(
            "rule1",
            "Allow Rule",
            "An allow rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::Network(NetworkObject {
                host: "example.com".to_string(),
                port: Some(80),
                protocol: None,
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
            PolicyObject::Network(NetworkObject {
                host: "evil.com".to_string(),
                port: Some(80),
                protocol: None,
            }),
            PolicyAction::Deny,
            None,
            0,
        );

        // Add the rules to the store
        policy_store.add_rule(rule1).unwrap();
        policy_store.add_rule(rule2).unwrap();

        // Check if a request is allowed
        let request = AccessRequest::Network {
            host: "example.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(evaluator.is_allowed(&plugin_id, &request).unwrap());

        // Check if a request is denied
        let request = AccessRequest::Network {
            host: "evil.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(!evaluator.is_allowed(&plugin_id, &request).unwrap());

        // Check if a request with no matching policy is denied
        let request = AccessRequest::Network {
            host: "unknown.com".to_string(),
            port: 80,
            connect: true,
            listen: false,
        };
        assert!(!evaluator.is_allowed(&plugin_id, &request).unwrap());
    }

    #[test]
    fn test_evaluation_cache() {
        // Create a policy store
        let policy_store = InMemoryPolicyStore::new();
        let mut evaluator = PolicyEvaluator::new(policy_store.clone());

        // Create a plugin
        let plugin_id = PluginId::new();

        // Create a policy rule
        let rule = PolicyRule::new(
            "rule1",
            "Allow Rule",
            "An allow rule",
            PolicySubject::Plugin(plugin_id.clone()),
            PolicyObject::PluginCall(PluginCallObject {
                plugin_id: "target".to_string(),
                function: Some("function".to_string()),
            }),
            PolicyAction::Allow,
            None,
            0,
        );

        // Add the rule to the store
        policy_store.add_rule(rule).unwrap();

        // Evaluate a request
        let request = AccessRequest::PluginCall {
            plugin_id: "target".to_string(),
            function: "function".to_string(),
        };
        let evaluation = evaluator.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(evaluation.result, EvaluationResult::Allow));

        // Remove the rule from the store
        policy_store.remove_rule("rule1").unwrap();

        // Evaluate the same request; should still return the cached result
        let evaluation = evaluator.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(evaluation.result, EvaluationResult::Allow));

        // Clear the cache
        evaluator.clear_cache();

        // Evaluate the request again; should return NoPolicy
        let evaluation = evaluator.evaluate(&plugin_id, &request).unwrap();
        assert!(matches!(evaluation.result, EvaluationResult::NoPolicy));
    }
}
