//! WebAssembly runtime integration for Lion Plugin System.
//!
//! This crate provides the implementation for loading and executing
//! WebAssembly plugins using the Wasmtime runtime.

pub mod error;
pub mod host_functions;
pub mod instance;
pub mod module;
pub mod runtime;

// Re-exports for convenience
pub use error::WasmRuntimeError;
pub use instance::{WasmInstance, WasmInstanceConfig};
pub use module::WasmModule;
pub use runtime::{WasmRuntime, WasmRuntimeConfig};