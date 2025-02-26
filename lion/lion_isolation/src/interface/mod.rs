//! Plugin interfaces.
//!
//! This module provides interfaces for plugins.

mod capability;
mod default_capability_checker;

pub use capability::{CapabilityChecker, CapabilityInterface};
pub use default_capability_checker::DefaultCapabilityChecker;
