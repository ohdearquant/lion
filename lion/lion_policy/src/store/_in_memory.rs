//! In-memory policy store.
//! 
//! This module provides an in-memory implementation of the policy store.

use std::sync::Arc;
use dashmap::DashMap;
use chrono::Utc;
use lion_core::error::{Result, PolicyError};

use crate::model::PolicyRule;
use super::PolicyStore;

/// An in-memory policy store.
#[derive(Clone)]
pub struct InMemoryPolicyStore {
    /// The rules, indexed by ID.
    rules: Arc<DashMap<String, PolicyRule>>,
}

impl InMemoryPolicyStore {
    /// Create a new in-memory policy store.
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InMemoryPolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyStore for InMemoryPolicyStore {
    fn add_rule(&self, rule: PolicyRule) -> Result<()> {
        // Check if the rule already exists
        if self.rules.contains_key(&rule.id) {
            return Err(PolicyError::RuleNotFound(format!("Rule {} already exists", rule.id)).into());
        }
        
        // Add the rule
        self.rules.insert(rule.id.clone(), rule);
        
        Ok(())
    }
    
    fn get_rule(&self, rule_id: &str) -> Result<PolicyRule> {
        // Get the rule
        let rule = self.rules.get(rule_id)
            .ok_or_else(|| PolicyError::RuleNotFound(rule_id.to_string()))?
            .clone();
        
        Ok(rule)
    }
    
    fn update_rule(&self, mut rule: PolicyRule) -> Result<()> {
        // Check if the rule exists
        if !self.rules.contains_key(&rule.id) {
            return Err(PolicyError::RuleNotFound(rule.id.clone()).into());
        }
        
        // Update the rule's timestamp
        rule.updated_at = Utc::now();
        
        // Update the rule
        self.rules.insert(rule.id.clone(), rule);
        
        Ok(())
    }
    
    fn remove_rule(&self, rule_id: &str) -> Result<()> {
        // Remove the rule
        if self.rules.remove(rule_id).is_none() {
            return Err(PolicyError::RuleNotFound(rule_id.to_string()).into());
        }
        
        Ok(())
    }
    
    fn list_rules(&self) -> Result<Vec<PolicyRule>> {
        // Get all rules
        let rules = self.rules.iter()
            .map(|r| r.value().clone())
            .collect();
        
        Ok(rules)
    }
    
    fn list_rules_matching<F>(&self, matcher: F) -> Result<Vec<PolicyRule>>
    where
        F: Fn(&PolicyRule) -> bool,
    {
        // Get matching rules
        let rules = self.rules.iter()
            .map(|r| r.value().clone())
            .filter(|r| matcher(r))
            .collect();
        
        Ok(rules)
    }
    
    fn clear_rules(&self) -> Result<()> {
        // Clear all rules
        self.rules.clear();
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PolicySubject, PolicyObject, PolicyAction};
    
    #[test]
    fn test_add_and_get_rule() {
        let store = InMemoryPolicyStore::new();
        let rule = PolicyRule::new(
            "rule1",
            "Test Rule",
            "A test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        // Add the rule
        store.add_rule(rule.clone()).unwrap();
        
        // Get the rule
        let retrieved = store.get_rule(&rule.id).unwrap();
        
        assert_eq!(retrieved.id, rule.id);
        assert_eq!(retrieved.name, rule.name);
    }
    
    #[test]
    fn test_update_rule() {
        let store = InMemoryPolicyStore::new();
        let rule = PolicyRule::new(
            "rule1",
            "Test Rule",
            "A test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        // Add the rule
        store.add_rule(rule.clone()).unwrap();
        
        // Update the rule
        let mut updated = rule.clone();
        updated.name = "Updated Rule".to_string();
        store.update_rule(updated.clone()).unwrap();
        
        // Get the rule
        let retrieved = store.get_rule(&rule.id).unwrap();
        
        assert_eq!(retrieved.id, rule.id);
        assert_eq!(retrieved.name, "Updated Rule");
    }
    
    #[test]
    fn test_remove_rule() {
        let store = InMemoryPolicyStore::new();
        let rule = PolicyRule::new(
            "rule1",
            "Test Rule",
            "A test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        
        // Add the rule
        store.add_rule(rule.clone()).unwrap();
        
        // Remove the rule
        store.remove_rule(&rule.id).unwrap();
        
        // Try to get the rule
        let result = store.get_rule(&rule.id);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_list_rules() {
        let store = InMemoryPolicyStore::new();
        let rule1 = PolicyRule::new(
            "rule1",
            "Test Rule 1",
            "A test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        let rule2 = PolicyRule::new(
            "rule2",
            "Test Rule 2",
            "Another test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Deny,
            None,
            0,
        );
        
        // Add the rules
        store.add_rule(rule1.clone()).unwrap();
        store.add_rule(rule2.clone()).unwrap();
        
        // List the rules
        let rules = store.list_rules().unwrap();
        
        assert_eq!(rules.len(), 2);
        assert!(rules.iter().any(|r| r.id == "rule1"));
        assert!(rules.iter().any(|r| r.id == "rule2"));
    }
    
    #[test]
    fn test_list_rules_matching() {
        let store = InMemoryPolicyStore::new();
        let rule1 = PolicyRule::new(
            "rule1",
            "Test Rule 1",
            "A test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        let rule2 = PolicyRule::new(
            "rule2",
            "Test Rule 2",
            "Another test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Deny,
            None,
            0,
        );
        
        // Add the rules
        store.add_rule(rule1.clone()).unwrap();
        store.add_rule(rule2.clone()).unwrap();
        
        // List rules matching a condition
        let rules = store.list_rules_matching(|r| matches!(r.action, PolicyAction::Allow)).unwrap();
        
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "rule1");
    }
    
    #[test]
    fn test_clear_rules() {
        let store = InMemoryPolicyStore::new();
        let rule1 = PolicyRule::new(
            "rule1",
            "Test Rule 1",
            "A test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );
        let rule2 = PolicyRule::new(
            "rule2",
            "Test Rule 2",
            "Another test rule",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Deny,
            None,
            0,
        );
        
        // Add the rules
        store.add_rule(rule1.clone()).unwrap();
        store.add_rule(rule2.clone()).unwrap();
        
        // Clear the rules
        store.clear_rules().unwrap();
        
        // List the rules
        let rules = store.list_rules().unwrap();
        
        assert_eq!(rules.len(), 0);
    }
}