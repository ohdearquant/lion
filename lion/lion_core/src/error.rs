//! Error types for the Lion microkernel system.
//! 
//! This module defines a comprehensive error hierarchy that enables
//! precise error handling throughout the system.

use thiserror::Error;
use crate::id::{PluginId, CapabilityId, WorkflowId, NodeId, ExecutionId, RegionId, MessageId};

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
    
    #[error("Distribution error: {0}")]
    Distribution(#[from] DistributionError),
    
    #[error("Observability error: {0}")]
    Observability(#[from] ObservabilityError),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
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
    
    #[error("Plugin is in invalid state: {0:?}")]
    InvalidState(crate::types::PluginState),
    
    #[error("Plugin is paused")]
    Paused,
    
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
    #[error("Capability not found: {0}")]
    NotFound(CapabilityId),
    
    #[error("Capability not granted: {0}")]
    NotGranted(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Invalid capability: {0}")]
    Invalid(String),
    
    #[error("Capability revocation failed: {0}")]
    RevocationFailed(String),
    
    #[error("Capability constraint error: {0}")]
    ConstraintError(String),
    
    #[error("Capability composition error: {0}")]
    CompositionError(String),
}

/// Errors related to policy enforcement.
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Policy rule not found: {0}")]
    RuleNotFound(String),
    
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
    #[error("Plugin not loaded: {0}")]
    PluginNotLoaded(PluginId),
    
    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),
    
    #[error("Failed to compile module: {0}")]
    CompilationFailed(String),
    
    #[error("Failed to instantiate module: {0}")]
    InstantiationFailed(String),
    
    #[error("Failed to link host functions: {0}")]
    LinkingFailed(String),
    
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
    
    #[error("Acquisition timeout after {0}ms for plugin {1}")]
    AcquisitionTimeout(u64, PluginId),
    
    #[error("Instance pool limit reached: {0}")]
    PoolLimitReached(String),
    
    #[error("Actor initialization failed: {0}")]
    ActorInitFailed(String),
    
    #[error("Actor message delivery failed: {0}")]
    MessageDeliveryFailed(String),
    
    #[error("Supervisor error: {0}")]
    SupervisorError(String),
}

/// Errors related to workflow operations.
#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
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
    
    #[error("Execution not found: {0}")]
    ExecutionNotFound(String),
    
    #[error("Persistence error: {0}")]
    PersistenceError(String),
}

/// Errors related to distribution operations.
#[derive(Debug, Error)]
pub enum DistributionError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    
    #[error("Communication failed: {0}")]
    CommunicationFailed(String),
    
    #[error("Capability export failed: {0}")]
    CapabilityExportFailed(String),
    
    #[error("Capability import failed: {0}")]
    CapabilityImportFailed(String),
    
    #[error("Token validation failed: {0}")]
    TokenValidationFailed(String),
    
    #[error("Remote call failed: {0}")]
    RemoteCallFailed(String),
    
    #[error("Cluster membership error: {0}")]
    MembershipError(String),
}

/// Errors related to observability operations.
#[derive(Debug, Error)]
pub enum ObservabilityError {
    #[error("Tracing initialization failed: {0}")]
    TracingInitFailed(String),
    
    #[error("Metrics initialization failed: {0}")]
    MetricsInitFailed(String),
    
    #[error("Logging initialization failed: {0}")]
    LoggingInitFailed(String),
    
    #[error("Context propagation failed: {0}")]
    ContextPropagationFailed(String),
}

/// Result type used throughout the Lion system.
pub type Result<T> = std::result::Result<T, Error>;