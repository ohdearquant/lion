//! WebAssembly module loading and compilation.

use crate::error::WasmIsolationError;
use lion_core::plugin::PluginSource;
use std::path::Path;
use std::sync::Arc;
use wasmtime::{Engine, Module};

/// A compiled WebAssembly module ready for instantiation
#[derive(Clone)]
pub struct WasmModule {
    /// The compiled Wasmtime module
    module: Arc<Module>,
    
    /// The name of the module
    name: String,
}

impl WasmModule {
    /// Load and compile a WebAssembly module from a plugin source
    pub fn from_source(
        engine: &Engine,
        source: &PluginSource,
        name: &str,
    ) -> Result<Self, WasmIsolationError> {
        let module = match source {
            PluginSource::FilePath(path) => Self::from_file(engine, path)?,
            PluginSource::InMemory(bytes) => Self::from_binary(engine, bytes)?,
            PluginSource::Url(url) => Self::from_url(engine, url)?,
        };
        
        Ok(Self {
            module,
            name: name.to_string(),
        })
    }
    
    /// Load and compile a WebAssembly module from a file
    pub fn from_file(engine: &Engine, path: &Path) -> Result<Arc<Module>, WasmIsolationError> {
        let bytes = std::fs::read(path)
            .map_err(|e| WasmIsolationError::Io(format!("Failed to read file: {}", e)))?;
        
        Self::from_binary(engine, &bytes)
    }
    
    /// Load and compile a WebAssembly module from binary data
    pub fn from_binary(engine: &Engine, bytes: &[u8]) -> Result<Arc<Module>, WasmIsolationError> {
        // Try to compile as binary WebAssembly
        let module = Module::new(engine, bytes)
            .or_else(|_| {
                // If that failed, try to parse as WAT (text format)
                let binary = wat::parse_bytes(bytes)
                    .map_err(|e| WasmIsolationError::InvalidWebAssembly(e.to_string()))?;
                Module::new(engine, &binary)
                    .map_err(|e| WasmIsolationError::CompilationFailed(e.to_string()))
            })
            .map_err(|e| WasmIsolationError::CompilationFailed(e.to_string()))?;
        
        Ok(Arc::new(module))
    }
    
    /// Load and compile a WebAssembly module from a URL
    pub fn from_url(engine: &Engine, url: &str) -> Result<Arc<Module>, WasmIsolationError> {
        // Simple synchronous HTTP GET for the MVP
        // In a production system, this should be asynchronous
        let response = ureq::get(url)
            .call()
            .map_err(|e| WasmIsolationError::Io(format!("Failed to fetch from URL: {}", e)))?;
        
        let bytes = response
            .into_reader()
            .bytes()
            .collect::<Result<Vec<u8>, _>>()
            .map_err(|e| WasmIsolationError::Io(format!("Failed to read response body: {}", e)))?;
        
        Self::from_binary(engine, &bytes)
    }
    
    /// Get the compiled module
    pub fn module(&self) -> &Module {
        &self.module
    }
    
    /// Get the name of the module
    pub fn name(&self) -> &str {
        &self.name
    }
}