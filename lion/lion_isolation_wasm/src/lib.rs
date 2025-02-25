//! # Lion Isolation WASM
//!
//! This crate provides WebAssembly-based isolation for the Lion Plugin System
//! using the Wasmtime runtime. It implements the `IsolationBackend` trait from
//! the `lion_core` crate, allowing plugins to be safely executed in isolated
//! WebAssembly environments.
//!
//! ## Features
//!
//! - Safe execution of WebAssembly plugins with resource limits
//! - Host functions for file system, network, and inter-plugin communication
//! - Memory and execution time monitoring
//! - Support for loading modules from files, URLs, or memory

pub mod backend;
pub mod config;
pub mod error;
pub mod host_functions;
pub mod instance;
pub mod memory;
pub mod module;

// Re-exports for convenience
pub use backend::WasmIsolationBackend;
pub use config::{WasmInstanceConfig, WasmIsolationConfig};
pub use error::WasmIsolationError;
pub use instance::WasmInstance;
pub use module::WasmModule;

use lion_core::capability::CapabilityManager;
use lion_core::message::MessageBus;
use std::sync::Arc;

/// Create a new WebAssembly isolation backend with the default configuration
pub fn create_wasm_backend(
    capability_manager: Arc<dyn CapabilityManager>,
    message_bus: Arc<dyn MessageBus>,
) -> Result<Arc<WasmIsolationBackend>, error::WasmIsolationError> {
    let config = config::WasmIsolationConfig::default();
    WasmIsolationBackend::new(capability_manager, message_bus, config)
        .map(Arc::new)
}