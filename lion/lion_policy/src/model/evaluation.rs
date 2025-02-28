//! Policy evaluation model.
//!
//! This module defines policy evaluation types.

use chrono::{DateTime, Utc};
use lion_core::id::PluginId;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::model::{Constraint, PolicyAction, PolicyRule};

/// A policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    /// The plugin ID.
    pub plugin_id: PluginId,

    /// The access request.
    pub request: lion_core::types::AccessRequest,

    /// The result of the evaluation.
    pub result: EvaluationResult,

    /// The rule that was matched.
    pub matched_rule: Option<PolicyRule>,

    /// When the evaluation was performed.
    pub timestamp: DateTime<Utc>,
}

impl Evaluation {
    /// Create a new policy evaluation.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `request` - The access request.
    /// * `result` - The result of the evaluation.
    /// * `matched_rule` - The rule that was matched.
    ///
    /// # Returns
    ///
    /// A new policy evaluation.
    pub fn new(
        plugin_id: PluginId,
        request: lion_core::types::AccessRequest,
        result: EvaluationResult,
        matched_rule: Option<PolicyRule>,
    ) -> Self {
        Self {
            plugin_id,
            request,
            result,
            matched_rule,
            timestamp: Utc::now(),
        }
    }
}

/// The result of a policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvaluationResult {
    /// The action is allowed.
    Allow,

    /// The action is denied.
    Deny,

    /// The action is allowed with constraints.
    AllowWithConstraints(Vec<Constraint>),

    /// No matching policy was found.
    NoPolicy,

    /// The action should be audited.
    Audit,
}

impl fmt::Display for EvaluationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allow => write!(f, "Allow"),
            Self::Deny => write!(f, "Deny"),
            Self::AllowWithConstraints(constraints) => {
                write!(f, "Allow with constraints [")?;
                for (i, constraint) in constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", constraint)?;
                }
                write!(f, "]")
            }
            Self::NoPolicy => write!(f, "No policy"),
            Self::Audit => write!(f, "Audit"),
        }
    }
}

impl From<&PolicyAction> for EvaluationResult {
    fn from(action: &PolicyAction) -> Self {
        match action {
            PolicyAction::Allow => Self::Allow,
            PolicyAction::Deny => Self::Deny,
            PolicyAction::AllowWithConstraints(constraints) => {
                let constraints = constraints
                    .iter()
                    .map(|s| crate::model::Constraint::Custom {
                        constraint_type: "from_string".to_string(),
                        value: s.clone(),
                    })
                    .collect();

                Self::AllowWithConstraints(constraints)
            }
            PolicyAction::TransformToConstraints(constraints) => {
                let constraints = constraints
                    .iter()
                    .map(|s| crate::model::Constraint::Custom {
                        constraint_type: "from_string".to_string(),
                        value: s.clone(),
                    })
                    .collect();

                Self::AllowWithConstraints(constraints)
            }
            PolicyAction::Audit => Self::Audit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluation_new() {
        let plugin_id = PluginId::new();
        let request = lion_core::types::AccessRequest::File {
            path: std::path::PathBuf::from("/tmp/file"),
            read: true,
            write: false,
            execute: false,
        };
        let result = EvaluationResult::Allow;

        let evaluation = Evaluation::new(plugin_id.clone(), request.clone(), result, None);

        assert_eq!(evaluation.plugin_id, plugin_id);
        assert!(matches!(
            evaluation.request,
            lion_core::types::AccessRequest::File { .. }
        ));
        assert!(matches!(evaluation.result, EvaluationResult::Allow));
        assert!(evaluation.matched_rule.is_none());
    }

    #[test]
    fn test_evaluation_result_from_policy_action() {
        let action = PolicyAction::Allow;
        let result = EvaluationResult::from(&action);
        assert!(matches!(result, EvaluationResult::Allow));

        let action = PolicyAction::Deny;
        let result = EvaluationResult::from(&action);
        assert!(matches!(result, EvaluationResult::Deny));

        let action = PolicyAction::Audit;
        let result = EvaluationResult::from(&action);
        assert!(matches!(result, EvaluationResult::Audit));

        let action = PolicyAction::AllowWithConstraints(vec!["file_path:/tmp".to_string()]);
        let result = EvaluationResult::from(&action);

        match result {
            EvaluationResult::AllowWithConstraints(constraints) => {
                assert_eq!(constraints.len(), 1);

                match &constraints[0] {
                    Constraint::Custom {
                        constraint_type,
                        value,
                    } => {
                        assert_eq!(constraint_type, "from_string");
                        assert_eq!(value, "file_path:/tmp");
                    }
                    _ => panic!("Unexpected constraint type"),
                }
            }
            _ => panic!("Unexpected result type"),
        }
    }
}
