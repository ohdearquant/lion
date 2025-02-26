//! WebAssembly isolation.
//! 
//! This module provides isolation using WebAssembly.

pub mod engine;
pub mod module;
pub mod memory;
pub mod hostcall;

pub use engine::WasmEngine;
pub use module::WasmModule;
pub use memory::WasmMemory;
pub use hostcall::HostCallContext;