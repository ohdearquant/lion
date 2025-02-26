//! Module storage for caching compiled WebAssembly modules.
//!
//! This module provides storage for compiled WebAssembly modules,
//! reducing the overhead of recompiling modules for each plugin.

use std::sync::Arc;
use std::collections::HashMap;

use dashmap::DashMap;
use parking_lot::RwLock;

use lion_core::error::{Result, IsolationError};
use crate::wasm::WasmModule;

/// Interface for module storage.
pub trait ModuleStore: Send + Sync {
    /// Add a module to the store.
    fn add_module(&self, code: &[u8], module: Arc<WasmModule>) -> Result<()>;
    
    /// Get a module from the store.
    fn get_module(&self, code: &[u8]) -> Option<Arc<WasmModule>>;
    
    /// Remove a module from the store.
    fn remove_module(&self, code: &[u8]) -> Result<()>;
    
    /// Clear the store.
    fn clear(&self) -> Result<()>;
}

/// In-memory module store.
pub struct MemoryModuleStore {
    /// Storage for modules, keyed by code hash.
    modules: DashMap<String, Arc<WasmModule>>,
}

impl MemoryModuleStore {
    /// Create a new in-memory module store.
    pub fn new() -> Self {
        Self {
            modules: DashMap::new(),
        }
    }
    
    /// Compute the hash of a code block.
    fn compute_hash(code: &[u8]) -> String {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write(code);
        format!("{:016x}", hasher.finish())
    }
}

impl ModuleStore for MemoryModuleStore {
    fn add_module(&self, code: &[u8], module: Arc<WasmModule>) -> Result<()> {
        let hash = Self::compute_hash(code);
        self.modules.insert(hash, module);
        Ok(())
    }
    
    fn get_module(&self, code: &[u8]) -> Option<Arc<WasmModule>> {
        let hash = Self::compute_hash(code);
        self.modules.get(&hash).map(|entry| {
            let module = entry.value();
            module.increment_ref();
            module.clone()
        })
    }
    
    fn remove_module(&self, code: &[u8]) -> Result<()> {
        let hash = Self::compute_hash(code);
        self.modules.remove(&hash);
        Ok(())
    }
    
    fn clear(&self) -> Result<()> {
        self.modules.clear();
        Ok(())
    }
}