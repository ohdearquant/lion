//! Capability management for the Lion runtime
//!
//! This module provides components for managing capabilities, including
//! granting, checking, and revoking capabilities.

pub mod manager;
pub mod resolution;

// Re-export key types for convenience
pub use manager::CapabilityManager;