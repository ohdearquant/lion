//! WebAssembly module.
//!
//! This module provides a wrapper for a WebAssembly module.

use wasmtime::{Linker, Module};

use crate::wasm::hostcall::HostCallContext;

/// A WebAssembly module.
pub struct WasmModule {
    /// The wasmtime module.
    pub(crate) module: Module,

    /// The linker.
    pub(crate) linker: Linker<HostCallContext>,
}

impl WasmModule {
    /// Get the underlying Wasmtime module.
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Get the linker.
    pub fn linker(&self) -> &Linker<HostCallContext> {
        &self.linker
    }

    /// Get a mutable reference to the linker.
    pub fn linker_mut(&mut self) -> &mut Linker<HostCallContext> {
        &mut self.linker
    }
}
