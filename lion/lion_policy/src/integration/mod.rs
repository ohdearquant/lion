//! Policy-capability integration.
//!
//! This module provides functionality for integrating policies with capabilities.

mod mapper;
mod resolver;

pub use mapper::CapabilityMapper;
pub use resolver::ConstraintResolver;
