//! WebAssembly isolation.
//!
//! This module provides isolation using WebAssembly.

pub mod engine;
pub mod hostcall;
pub mod memory;
pub mod module;

pub use engine::WasmEngine;
pub use hostcall::HostCallContext;
pub use memory::WasmMemory;
pub use module::WasmModule;
