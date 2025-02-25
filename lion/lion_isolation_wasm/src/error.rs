//! Error types for the WebAssembly isolation backend.

use thiserror::Error;

/// Errors that can occur in the WebAssembly isolation backend
#[derive(Error, Debug)]
pub enum WasmIsolationError {
    #[error("Failed to compile WebAssembly module: {0}")]
    CompilationFailed(String),
    
    #[error("Failed to instantiate WebAssembly module: {0}")]
    InstantiationFailed(String),
    
    #[error("Function not found in WebAssembly module: {0}")]
    FunctionNotFound(String),
    
    #[error("Memory not found in WebAssembly module")]
    MemoryNotFound,
    
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    
    #[error("Execution error: {0}")]
    ExecutionError(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Trap occurred: {0}")]
    Trap(String),
    
    #[error("Wasmtime error: {0}")]
    Wasmtime(String),
    
    #[error("Invalid WebAssembly: {0}")]
    InvalidWebAssembly(String),
    
    #[error("Host function error: {0}")]
    HostFunction(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("I/O error: {0}")]
    Io(String),
    
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    
    #[error("Capability error: {0}")]
    CapabilityError(String),
}

impl From<wasmtime::Error> for WasmIsolationError {
    fn from(err: wasmtime::Error) -> Self {
        if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
            return Self::Trap(trap.to_string());
        }
        Self::Wasmtime(err.to_string())
    }
}

impl From<anyhow::Error> for WasmIsolationError {
    fn from(err: anyhow::Error) -> Self {
        if let Some(err) = err.downcast_ref::<wasmtime::Error>() {
            return Self::from(err.clone());
        }
        Self::Wasmtime(err.to_string())
    }
}

impl From<std::io::Error> for WasmIsolationError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for WasmIsolationError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<lion_core::error::CapabilityError> for WasmIsolationError {
    fn from(err: lion_core::error::CapabilityError) -> Self {
        Self::CapabilityError(err.to_string())
    }
}

impl From<WasmIsolationError> for lion_core::error::IsolationError {
    fn from(err: WasmIsolationError) -> Self {
        match err {
            WasmIsolationError::CompilationFailed(msg) => 
                Self::InitializationFailed(format!("Compilation failed: {}", msg)),
            WasmIsolationError::InstantiationFailed(msg) => 
                Self::InitializationFailed(format!("Instantiation failed: {}", msg)),
            WasmIsolationError::FunctionNotFound(msg) => 
                Self::ExecutionFailed(format!("Function not found: {}", msg)),
            WasmIsolationError::MemoryNotFound => 
                Self::ExecutionFailed("Memory not found".to_string()),
            WasmIsolationError::TypeMismatch(msg) => 
                Self::ExecutionFailed(format!("Type mismatch: {}", msg)),
            WasmIsolationError::ExecutionError(msg) => 
                Self::ExecutionFailed(msg),
            WasmIsolationError::ResourceLimitExceeded(msg) => 
                Self::ResourceLimitExceeded(msg),
            WasmIsolationError::Trap(msg) => 
                Self::ExecutionFailed(format!("WebAssembly trap: {}", msg)),
            WasmIsolationError::PluginNotFound(msg) => 
                Self::ExecutionFailed(format!("Plugin not found: {}", msg)),
            _ => Self::BackendError(err.to_string()),
        }
    }
}