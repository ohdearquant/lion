//! Interfaces to Lion microkernel components
//!
//! This module contains interfaces to connect the CLI with the actual
//! Lion microkernel components like runtime, isolation, policy, etc.

pub mod capability;
pub mod isolation;
pub mod observability;
pub mod policy;
pub mod runtime;
pub mod workflow;
