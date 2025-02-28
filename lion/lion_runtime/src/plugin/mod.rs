//! Plugin management for the Lion runtime
//!
//! This module provides components for managing plugins, including
//! lifecycle management, registration, and execution.

pub mod lifecycle;
pub mod manager;
pub mod registry;

// Re-export key types for convenience
pub use manager::PluginManager;
