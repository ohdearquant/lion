//! Capability checking.
//! 
//! This module provides functionality for checking capabilities.

mod engine;
mod aggregator;
mod audit;

pub use engine::CapabilityChecker;
pub use aggregator::CapabilityAggregator;
pub use audit::AuditLog;