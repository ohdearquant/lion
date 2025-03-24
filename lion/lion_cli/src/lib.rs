//! Lion Command Line Interface Library
//!
//! This library provides the functionality for the Lion CLI,
//! which is used to interact with the Lion microkernel system.
//!
//! It includes commands for:
//! - Plugin management (load, list, call, unload)
//! - System management (start, status, logs, shutdown)
//! - Workflow management (register, start, status, cancel)

pub mod commands;
pub mod integration;
pub mod interfaces;
