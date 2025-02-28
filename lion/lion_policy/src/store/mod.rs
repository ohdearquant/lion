//! Policy storage.
//!
//! This module provides storage for policies.

mod in_memory;

pub use in_memory::InMemoryPolicyStore;

use crate::model::PolicyRule;
use lion_core::error::Result;

/// Trait for policy storage.
///
/// A policy store is responsible for storing and retrieving policies.
pub trait PolicyStore: Send + Sync {
    /// Add a policy rule to the store.
    ///
    /// # Arguments
    ///
    /// * `rule` - The policy rule to add.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the rule was successfully added.
    /// * `Err` - If the rule could not be added.
    fn add_rule(&self, rule: PolicyRule) -> Result<()>;

    /// Get a policy rule from the store.
    ///
    /// # Arguments
    ///
    /// * `rule_id` - The ID of the rule to get.
    ///
    /// # Returns
    ///
    /// * `Ok(PolicyRule)` - The rule.
    /// * `Err` - If the rule could not be found.
    fn get_rule(&self, rule_id: &str) -> Result<PolicyRule>;

    /// Update a policy rule in the store.
    ///
    /// # Arguments
    ///
    /// * `rule` - The updated policy rule.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the rule was successfully updated.
    /// * `Err` - If the rule could not be updated.
    fn update_rule(&self, rule: PolicyRule) -> Result<()>;

    /// Remove a policy rule from the store.
    ///
    /// # Arguments
    ///
    /// * `rule_id` - The ID of the rule to remove.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the rule was successfully removed.
    /// * `Err` - If the rule could not be removed.
    fn remove_rule(&self, rule_id: &str) -> Result<()>;

    /// List all policy rules in the store.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PolicyRule>)` - The rules.
    /// * `Err` - If the rules could not be listed.
    fn list_rules(&self) -> Result<Vec<PolicyRule>>;

    /// List policy rules that match a given matcher function.
    ///
    /// # Arguments
    ///
    /// * `matcher` - A function that returns `true` for rules that match.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PolicyRule>)` - The matching rules.
    /// * `Err` - If the rules could not be listed.
    fn list_rules_matching<F>(&self, matcher: F) -> Result<Vec<PolicyRule>>
    where
        F: Fn(&PolicyRule) -> bool;

    /// Clear all policy rules from the store.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the rules were successfully cleared.
    /// * `Err` - If the rules could not be cleared.
    fn clear_rules(&self) -> Result<()>;
}
