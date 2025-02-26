//! Error types for the Lion runtime.
//!
//! This module defines a carefully structured error hierarchy that
//! enables precise error handling throughout the system.

use thiserror::Error;

use crate::types::{PluginId, MessageId, RegionId, PluginState};

/// Root error type for the Lion system.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),
    
    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),
    
    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),
    
    #[error("Isolation error: {0}")]
    Isolation(#[from] IsolationError),
    
    #[error("Concurrency error: {0}")]
    Concurrency(#[from] ConcurrencyError),
    
    #[error("Workflow error: {0}")]
    Workflow(#[from] WorkflowError),
    
    #[error("Messaging error: {0}")]
    Messaging(#[from] MessagingError),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Errors related to plugin operations.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(PluginId),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("Plugin execution error: {0}")]
    ExecutionError(String),
    
    #[error("Plugin is in invalid state: {0}")]
    InvalidState(PluginState),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Function call timed out after {0}ms")]
    Timeout(u64),
    
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Plugin termination failed: {0}")]
    TerminationFailed(String),
    
    #[error("Plugin is upgrading")]
    Upgrading,
}

/// Errors related to capability operations.
#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Capability not granted: {0}")]
    NotGranted(String),
    
    #[error("Invalid capability: {0}")]
    Invalid(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Capability revocation failed: {0}")]
    RevocationFailed(String),
}

/// Errors related to policy enforcement.
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Policy violation: {0}")]
    Violation(String),
    
    #[error("Policy evaluation failed: {0}")]
    EvaluationFailed(String),
    
    #[error("Network access policy violation: {0}")]
    NetworkAccessViolation(String),
    
    #[error("File access policy violation: {0}")]
    FileAccessViolation(String),
    
    #[error("Resource limit policy exceeded: {0}")]
    ResourceLimitExceeded(String),
}

/// Errors related to isolation operations.
#[derive(Debug, Error)]
pub enum IsolationError {
    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),
    
    #[error("Failed to compile module: {0}")]
    CompilationFailed(String),
    
    #[error("Failed to instantiate module: {0}")]
    InstantiationFailed(String),
    
    #[error("Execution trap: {0}")]
    ExecutionTrap(String),
    
    #[error("Invalid module format: {0}")]
    InvalidModuleFormat(String),
    
    #[error("Memory access error: {0}")]
    MemoryAccessError(String),
    
    #[error("Memory region not found: {0}")]
    RegionNotFound(RegionId),
}

/// Errors related to concurrency operations.
#[derive(Debug, Error)]
pub enum ConcurrencyError {
    #[error("Failed to create instance: {0}")]
    InstanceCreationFailed(String),
    
    #[error("No available instances for plugin: {0}")]
    NoAvailableInstances(PluginId),
    
    #[error("Thread pool exhausted")]
    ThreadPoolExhausted,
    
    #[error("Acquisition timeout after {0}ms")]
    AcquisitionTimeout(u64),
    
    #[error("Instance pool limit reached: {0}")]
    PoolLimitReached(String),
}

/// Errors related to workflow operations.
#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow definition error: {0}")]
    DefinitionError(String),
    
    #[error("Node execution failed: {0}")]
    NodeExecutionFailed(String),
    
    #[error("Workflow execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Workflow timeout after {0}ms")]
    Timeout(u64),
    
    #[error("Workflow cancelled")]
    Cancelled,
    
    #[error("Cyclic dependency detected")]
    CyclicDependency,
}

/// Errors related to messaging operations.
#[derive(Debug, Error)]
pub enum MessagingError {
    #[error("Message not found: {0}")]
    MessageNotFound(MessageId),
    
    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),
    
    #[error("Message queue full for plugin: {0}")]
    QueueFull(PluginId),
    
    #[error("Invalid recipient: {0}")]
    InvalidRecipient(PluginId),
    
    #[error("Message timeout after {0}ms")]
    Timeout(u64),
}

/// Result type used throughout the Lion system.
pub type Result<T> = std::result::Result<T, Error>;