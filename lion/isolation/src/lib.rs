//! Lion Isolation - Plugin isolation for the Lion runtime
//!
//! This crate provides isolation backends for running plugins securely.
//! It includes WebAssembly and native code execution environments.

mod wasm;
mod store;
mod backend;

pub use wasm::{WasmIsolation, WasmModule};
pub use store::{ModuleStore, MemoryModuleStore};
pub use backend::{IsolationBackend, WasmIsolationBackend};

use std::sync::Arc;

use lion_core::error::Result;
use lion_core::traits::IsolationBackendFactory;

/// Factory for creating WASM isolation backends.
pub struct WasmIsolationFactory {
    /// Module store for caching compiled modules.
    store: Arc<dyn ModuleStore>,
    
    /// Maximum memory usage in bytes.
    max_memory: usize,
}

impl WasmIsolationFactory {
    /// Create a new WASM isolation factory.
    pub fn new(max_memory: usize) -> Self {
        let store = Arc::new(MemoryModuleStore::new());
        Self { store, max_memory }
    }
    
    /// Set a custom module store.
    pub fn with_store(mut self, store: Arc<dyn ModuleStore>) -> Self {
        self.store = store;
        self
    }
}

impl IsolationBackendFactory for WasmIsolationFactory {
    fn create_backend(&self) -> Result<Arc<dyn lion_core::traits::IsolationBackend>> {
        let backend = WasmIsolationBackend::new(
            self.store.clone(),
            self.max_memory,
        )?;
        
        Ok(Arc::new(backend))
    }
}