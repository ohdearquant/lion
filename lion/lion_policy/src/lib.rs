//! # Lion Policy
//!
//! `lion_policy` provides a policy system for the Lion microkernel.
//! Policies define rules that constrain capabilities, providing
//! fine-grained control over resource access.
//!
//! Key concepts:
//!
//! 1. **Policy Rule**: A rule that specifies what actions are allowed or denied.
//!
//! 2. **Constraint**: A restriction applied to a capability.
//!
//! 3. **Policy Evaluation**: The process of checking if an action is allowed by policy.
//!
//! 4. **Policy-Capability Integration**: The process of applying policy constraints
//!    to capabilities.

pub mod engine;
pub mod error;
pub mod integration;
pub mod model;
pub mod store;

// Re-export key types and traits for convenience
pub use engine::{PolicyAggregator, PolicyAudit, PolicyEvaluator};
pub use integration::{CapabilityMapper, ConstraintResolver};
pub use model::{
    Constraint, PolicyAction, PolicyCondition, PolicyObject, PolicyRule, PolicySubject,
};
pub use store::{InMemoryPolicyStore, PolicyStore};
