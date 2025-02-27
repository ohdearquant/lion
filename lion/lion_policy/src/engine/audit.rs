//! Policy auditing.
//!
//! This module provides functionality for auditing policy evaluations.

use dashmap::DashMap;
use lion_core::error::Result;
use lion_core::id::PluginId;
use std::sync::Arc;

use crate::model::{Evaluation, EvaluationResult};

/// A policy audit.
///
/// This audit records policy evaluations.
#[derive(Clone)]
pub struct PolicyAudit {
    /// The audit entries.
    entries: Arc<DashMap<PluginId, Vec<Evaluation>>>,

    /// The maximum number of entries to keep per plugin.
    max_entries_per_plugin: usize,
}

impl PolicyAudit {
    /// Create a new policy audit.
    ///
    /// # Arguments
    ///
    /// * `max_entries_per_plugin` - The maximum number of entries to keep per plugin.
    ///
    /// # Returns
    ///
    /// A new policy audit.
    pub fn new(max_entries_per_plugin: usize) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            max_entries_per_plugin,
        }
    }

    /// Record an evaluation.
    ///
    /// # Arguments
    ///
    /// * `evaluation` - The evaluation to record.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the evaluation was successfully recorded.
    /// * `Err` - If the evaluation could not be recorded.
    pub fn record(&self, evaluation: Evaluation) -> Result<()> {
        // Add the evaluation to the plugin's entries
        self.entries
            .entry(evaluation.plugin_id.clone())
            .or_insert_with(Vec::new)
            .push(evaluation.clone());

        // Trim the entries if necessary
        if let Some(mut plugin_entries) = self.entries.get_mut(&evaluation.plugin_id.clone()) {
            if plugin_entries.len() > self.max_entries_per_plugin {
                let to_remove = plugin_entries.len() - self.max_entries_per_plugin;
                plugin_entries.drain(0..to_remove);
            }
        }

        Ok(())
    }

    /// Get evaluations for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to get evaluations for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Evaluation>)` - The evaluations.
    /// * `Err` - If the evaluations could not be retrieved.
    pub fn get_evaluations(&self, plugin_id: &PluginId) -> Result<Vec<Evaluation>> {
        // Get the evaluations for the plugin
        let evaluations = match self.entries.get(plugin_id) {
            Some(entries) => entries.clone(),
            None => Vec::new(),
        };

        Ok(evaluations)
    }

    /// Clear evaluations for a plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The ID of the plugin to clear evaluations for.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the evaluations were successfully cleared.
    /// * `Err` - If the evaluations could not be cleared.
    pub fn clear_evaluations(&self, plugin_id: &PluginId) -> Result<()> {
        // Remove the plugin's entries
        self.entries.remove(plugin_id);

        Ok(())
    }

    /// Get all evaluations.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Evaluation>)` - All evaluations.
    /// * `Err` - If the evaluations could not be retrieved.
    pub fn get_all_evaluations(&self) -> Result<Vec<Evaluation>> {
        // Get all evaluations
        let mut evaluations = Vec::new();

        for entry in self.entries.iter() {
            evaluations.extend(entry.value().clone());
        }

        Ok(evaluations)
    }

    /// Get evaluations filtered by result.
    ///
    /// # Arguments
    ///
    /// * `result` - The result to filter by.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Evaluation>)` - The filtered evaluations.
    /// * `Err` - If the evaluations could not be retrieved.
    pub fn get_evaluations_by_result(&self, result: &EvaluationResult) -> Result<Vec<Evaluation>> {
        // Get evaluations matching the result
        let mut evaluations = Vec::new();

        for entry in self.entries.iter() {
            for evaluation in entry.value() {
                if &evaluation.result == result {
                    evaluations.push(evaluation.clone());
                }
            }
        }

        Ok(evaluations)
    }

    /// Get evaluations filtered by rule.
    ///
    /// # Arguments
    ///
    /// * `rule_id` - The ID of the rule to filter by.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Evaluation>)` - The filtered evaluations.
    /// * `Err` - If the evaluations could not be retrieved.
    pub fn get_evaluations_by_rule(&self, rule_id: &str) -> Result<Vec<Evaluation>> {
        // Get evaluations matching the rule
        let mut evaluations = Vec::new();

        for entry in self.entries.iter() {
            for evaluation in entry.value() {
                if let Some(rule) = &evaluation.matched_rule {
                    if rule.id == rule_id {
                        evaluations.push(evaluation.clone());
                    }
                }
            }
        }

        Ok(evaluations)
    }
}

impl Default for PolicyAudit {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PolicyAction, PolicyObject, PolicyRule, PolicySubject};
    use lion_core::types::AccessRequest;
    use std::path::PathBuf;

    #[test]
    fn test_record_and_get_evaluations() {
        let audit = PolicyAudit::new(10);
        let plugin_id = PluginId::new();

        // Create an evaluation
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        let result = EvaluationResult::Allow;
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
        let evaluation = Evaluation::new(plugin_id.clone(), request, result, Some(rule));

        // Record the evaluation
        audit.record(evaluation.clone()).unwrap();

        // Get evaluations for the plugin
        let evaluations = audit.get_evaluations(&plugin_id).unwrap();

        // Check that there's one evaluation
        assert_eq!(evaluations.len(), 1);

        // Check the evaluation details
        let recorded = &evaluations[0];
        assert_eq!(recorded.plugin_id, plugin_id);
        assert!(matches!(recorded.result, EvaluationResult::Allow));
        assert!(recorded.matched_rule.is_some());
    }

    #[test]
    fn test_clear_evaluations() {
        let audit = PolicyAudit::new(10);
        let plugin_id = PluginId::new();

        // Create an evaluation
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        let result = EvaluationResult::Allow;
        let evaluation = Evaluation::new(plugin_id.clone(), request, result, None);

        // Record the evaluation
        audit.record(evaluation).unwrap();

        // Clear evaluations for the plugin
        audit.clear_evaluations(&plugin_id).unwrap();

        // Get evaluations for the plugin
        let evaluations = audit.get_evaluations(&plugin_id).unwrap();

        // Check that there are no evaluations
        assert_eq!(evaluations.len(), 0);
    }

    #[test]
    fn test_max_entries_per_plugin() {
        let audit = PolicyAudit::new(2);
        let plugin_id = PluginId::new();

        // Create evaluations
        for i in 0..3 {
            let request = AccessRequest::File {
                path: PathBuf::from(format!("/tmp/file{}", i)),
                read: true,
                write: false,
                execute: false,
            };
            let result = EvaluationResult::Allow;
            let evaluation = Evaluation::new(plugin_id.clone(), request, result, None);

            // Record the evaluation
            audit.record(evaluation).unwrap();
        }

        // Get evaluations for the plugin
        let evaluations = audit.get_evaluations(&plugin_id).unwrap();

        // Check that there are only two evaluations
        assert_eq!(evaluations.len(), 2);

        // Check that the oldest evaluation was removed
        assert!(matches!(
            &evaluations[0].request,
            AccessRequest::File { path, .. } if *path == PathBuf::from("/tmp/file1")
        ));
        assert!(matches!(
            &evaluations[1].request,
            AccessRequest::File { path, .. } if *path == PathBuf::from("/tmp/file2")
        ));
    }

    #[test]
    fn test_get_evaluations_by_result() {
        let audit = PolicyAudit::new(10);
        let plugin_id = PluginId::new();

        // Create an Allow evaluation
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        let result = EvaluationResult::Allow;
        let evaluation = Evaluation::new(plugin_id.clone(), request, result, None);

        // Record the evaluation
        audit.record(evaluation).unwrap();

        // Create a Deny evaluation
        let request = AccessRequest::File {
            path: PathBuf::from("/etc/passwd"),
            read: true,
            write: false,
            execute: false,
        };
        let result = EvaluationResult::Deny;
        let evaluation = Evaluation::new(plugin_id.clone(), request, result, None);

        // Record the evaluation
        audit.record(evaluation).unwrap();

        // Get evaluations by result
        let allow_evaluations = audit
            .get_evaluations_by_result(&EvaluationResult::Allow)
            .unwrap();
        let deny_evaluations = audit
            .get_evaluations_by_result(&EvaluationResult::Deny)
            .unwrap();

        // Check the evaluations
        assert_eq!(allow_evaluations.len(), 1);
        assert_eq!(deny_evaluations.len(), 1);

        assert!(matches!(
            allow_evaluations[0].result,
            EvaluationResult::Allow
        ));
        assert!(matches!(deny_evaluations[0].result, EvaluationResult::Deny));
    }

    #[test]
    fn test_get_evaluations_by_rule() {
        let audit = PolicyAudit::new(10);
        let plugin_id = PluginId::new();

        // Create rules
        let rule1 = PolicyRule::new(
            "rule1",
            "Rule 1",
            "Rule 1",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Allow,
            None,
            0,
        );

        let rule2 = PolicyRule::new(
            "rule2",
            "Rule 2",
            "Rule 2",
            PolicySubject::Any,
            PolicyObject::Any,
            PolicyAction::Deny,
            None,
            0,
        );

        // Create evaluations
        let request = AccessRequest::File {
            path: PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        let result = EvaluationResult::Allow;
        let evaluation = Evaluation::new(
            plugin_id.clone(),
            request.clone(),
            result,
            Some(rule1.clone()),
        );

        // Record the evaluation
        audit.record(evaluation).unwrap();

        let result = EvaluationResult::Deny;
        let evaluation = Evaluation::new(plugin_id.clone(), request, result, Some(rule2.clone()));

        // Record the evaluation
        audit.record(evaluation).unwrap();

        // Get evaluations by rule
        let rule1_evaluations = audit.get_evaluations_by_rule("rule1").unwrap();
        let rule2_evaluations = audit.get_evaluations_by_rule("rule2").unwrap();

        // Check the evaluations
        assert_eq!(rule1_evaluations.len(), 1);
        assert_eq!(rule2_evaluations.len(), 1);

        assert_eq!(
            rule1_evaluations[0].matched_rule.as_ref().unwrap().id,
            "rule1"
        );
        assert_eq!(
            rule2_evaluations[0].matched_rule.as_ref().unwrap().id,
            "rule2"
        );
    }
}
