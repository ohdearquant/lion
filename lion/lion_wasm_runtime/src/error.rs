//! Error types for the WebAssembly runtime.

use thiserror::Error;

/// Errors that can occur in the WebAssembly runtime
#[derive(Error, Debug)]
pub enum WasmRuntimeError {
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
}

impl From<wasmtime::Error> for WasmRuntimeError {
    fn from(err: wasmtime::Error) -> Self {
        if let Some(trap) = err.downcast_ref::<wasmtime::Trap>() {
            return Self::Trap(trap.to_string());
        }
        Self::Wasmtime(err.to_string())
    }
}

impl From<anyhow::Error> for WasmRuntimeError {
    fn from(err: anyhow::Error) -> Self {
        if let Some(err) = err.downcast_ref::<wasmtime::Error>() {
            return Self::from(err.clone());
        }
        Self::Wasmtime(err.to_string())
    }
}

impl From<std::io::Error> for WasmRuntimeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<serde_json::Error> for WasmRuntimeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<WasmRuntimeError> for lion_core::error::PluginError {
    fn from(err: WasmRuntimeError) -> Self {
        match err {
            WasmRuntimeError::CompilationFailed(msg) => Self::LoadFailure(msg),
            WasmRuntimeError::InstantiationFailed(msg) => Self::LoadFailure(msg),
            WasmRuntimeError::ExecutionError(msg) => Self::ExecutionError(msg),
            WasmRuntimeError::ResourceLimitExceeded(_) => Self::ExecutionError("Resource limit exceeded".to_string()),
            WasmRuntimeError::Trap(msg) => Self::ExecutionError(format!("WebAssembly trap: {}", msg)),
            _ => Self::ExecutionError(err.to_string()),
        }
    }
}