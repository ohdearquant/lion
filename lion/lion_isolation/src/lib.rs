//! # Lion Isolation
//!
//! `lion_isolation` provides an isolation system for the Lion microkernel.
//! It allows plugins to be executed in a secure sandbox, with controlled
//! resource usage and isolation from the host system.
//!
//! Key concepts:
//!
//! 1. **Isolation Backend**: An engine that provides plugin isolation.
//!
//! 2. **Module**: A compiled WebAssembly module.
//!
//! 3. **Instance**: A running instance of a WebAssembly module.
//!
//! 4. **Resource Limiter**: A mechanism for limiting the resources used by plugins.
//!
//! 5. **Host Functions**: Functions exposed to plugins by the host.
//!
//! 6. **Plugin Lifecycle**: Management of plugin states throughout their execution.
//!
//! 7. **Instance Pool**: Reuse of WebAssembly instances for better performance.

pub mod interface;
pub mod manager;
pub mod resource;
pub mod wasm;

// Re-export key types and traits for convenience
pub use interface::CapabilityInterface;
pub use manager::{
    DefaultIsolationBackend, InstancePool, IsolationBackend, IsolationManager, PluginLifecycle,
    PluginState, PooledInstance,
};
pub use resource::{DefaultResourceLimiter, ResourceLimiter, ResourceMetering, ResourceUsage};
pub use wasm::{HostCallContext, WasmEngine, WasmMemory, WasmModule};
