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

pub mod model;
pub mod store;
pub mod integration;
pub mod engine;

// Re-export key types and traits for convenience
pub use model::{PolicyRule, Constraint, PolicyAction, PolicySubject, PolicyObject, PolicyCondition};
pub use store::{PolicyStore, InMemoryPolicyStore};
pub use integration::{CapabilityMapper, ConstraintResolver};
pub use engine::{PolicyEvaluator, PolicyAggregator, PolicyAudit};