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

pub mod wasm;
pub mod resource;
pub mod interface;
pub mod manager;

// Re-export key types and traits for convenience
pub use wasm::{WasmEngine, WasmModule, WasmMemory, HostCallContext};
pub use resource::{ResourceLimiter, ResourceMetering, ResourceUsage};
pub use interface::CapabilityInterface;
pub use manager::{IsolationBackend, IsolationManager, InstancePool};