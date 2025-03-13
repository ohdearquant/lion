//! Command modules for the Lion CLI
//!
//! This module contains submodules for different command categories:
//! - plugin: Plugin management commands
//! - policy: Policy management commands
//! - system: System management commands
//! - workflow: Workflow management commands

// Re-export interfaces for the command modules
#[path = "../interfaces/mod.rs"]
pub mod interfaces;

// Re-export command modules
pub mod plugin;
pub mod policy;
pub mod system;
pub mod workflow;
