//! Policy evaluation engine.
//! 
//! This module provides functionality for evaluating policies.

mod evaluator;
mod aggregator;
mod audit;

pub use evaluator::PolicyEvaluator;
pub use aggregator::PolicyAggregator;
pub use audit::PolicyAudit;