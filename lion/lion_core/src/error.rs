//! Error types for the Lion WebAssembly Plugin System.

use thiserror::Error;

/// The main error type for the Lion system
#[derive(Error, Debug)]
pub enum Error {
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),
    
    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),
    
    #[error("Message error: {0}")]
    Message(#[from] MessageError),
    
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),
    
    #[error("Isolation error: {0}")]
    Isolation(#[from] IsolationError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Errors related to plugin operations
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Failed to load plugin: {0}")]
    LoadFailure(String),
    
    #[error("Plugin initialization failed: {0}")]
    InitializationFailure(String),
    
    #[error("Plugin not found: {0}")]
    NotFound(String),
    
    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),
    
    #[error("Plugin execution error: {0}")]
    ExecutionError(String),
    
    #[error("Plugin timeout")]
    Timeout,
    
    #[error("Plugin already loaded")]
    AlreadyLoaded,
}

/// Errors related to capability operations
#[derive(Error, Debug)]
pub enum CapabilityError {
    #[error("Capability already granted")]
    AlreadyGranted,
    
    #[error("Capability not granted")]
    NotGranted,
    
    #[error("Permission denied for capability operation")]
    PermissionDenied,
    
    #[error("Unknown capability type")]
    UnknownCapability,
    
    #[error("Unknown plugin: {0}")]
    UnknownPlugin(String),
    
    #[error("Capability operation failed: {0}")]
    OperationFailed(String),
}

/// Errors related to messaging operations
#[derive(Error, Debug)]
pub enum MessageError {
    #[error("No such plugin")]
    NoSuchPlugin,
    
    #[error("No such topic")]
    NoSuchTopic,
    
    #[error("Permission denied for messaging operation")]
    PermissionDenied,
    
    #[error("Message bus is full")]
    BusFull,
    
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
    
    #[error("Message format error: {0}")]
    FormatError(String),
}

/// Errors related to resource monitoring
#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Resource limit exceeded: {0}")]
    LimitExceeded(String),
    
    #[error("Resource not available")]
    NotAvailable,
    
    #[error("Failed to monitor resource: {0}")]
    MonitoringFailed(String),
}

/// Errors related to isolation backends
#[derive(Error, Debug)]
pub enum IsolationError {
    #[error("Failed to initialize isolation backend: {0}")]
    InitializationFailed(String),
    
    #[error("Failed to execute code: {0}")]
    ExecutionFailed(String),
    
    #[error("Isolation violation: {0}")]
    ViolationDetected(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Isolation backend error: {0}")]
    BackendError(String),
}