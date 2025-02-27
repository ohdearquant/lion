//! System management for the Lion runtime
//!
//! This module provides components for system bootstrap, configuration,
//! and shutdown operations.

pub mod bootstrap;
pub mod config;
pub mod shutdown;

// Re-export key types for convenience
pub use bootstrap::System;
pub use config::RuntimeConfig;