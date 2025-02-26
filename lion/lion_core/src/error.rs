//! Error types for the Lion microkernel system.
//!
//! This module defines a comprehensive error hierarchy that enables
//! precise error handling throughout the system. The errors are organized
//! by subsystem, with each subsystem having its own error type.
//!
//! The root error type, `Error`, can wrap any of the subsystem-specific
//! errors, allowing for uniform error handling at the top level.

use crate::id::{CapabilityId, ExecutionId, MessageId, NodeId, PluginId, RegionId, WorkflowId};
use thiserror::Error;

/// Root error type for the Lion system.
#[derive(Debug, Error)]
pub enum Error {
    /// Plugin-related errors
    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),

    /// Capability-related errors
    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    /// Policy enforcement errors
    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),

    /// Isolation-related errors
    #[error("Isolation error: {0}")]
    Isolation(#[from] IsolationError),

    /// Concurrency and actor system errors
    #[error("Concurrency error: {0}")]
    Concurrency(#[from] ConcurrencyError),

    /// Workflow execution errors
    #[error("Workflow error: {0}")]
    Workflow(#[from] WorkflowError),

    /// Distributed system errors
    #[error("Distribution error: {0}")]
    Distribution(#[from] DistributionError),

    /// Logging, tracing, and telemetry errors
    #[error("Observability error: {0}")]
    Observability(#[from] ObservabilityError),

    /// General runtime errors
    #[error("Runtime error: {0}")]
    Runtime(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Unimplemented features
    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

/// Errors related to plugin operations.
#[derive(Debug, Error)]
pub enum PluginError {
    /// Plugin with the given ID was not found
    #[error("Plugin not found: {0}")]
    NotFound(PluginId),

    /// Function with the given name was not found in the plugin
    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    /// Plugin execution failed with the given error
    #[error("Plugin execution error: {0}")]
    ExecutionError(String),

    /// Plugin is in an invalid state for the requested operation
    #[error("Plugin is in invalid state: {0:?}")]
    InvalidState(crate::types::PluginState),

    /// Plugin is paused and cannot process requests
    #[error("Plugin is paused")]
    Paused,

    /// Plugin exceeded a resource limit
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    /// Function call timed out
    #[error("Function call timed out after {0}ms")]
    Timeout(u64),

    /// Plugin initialization failed
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    /// Plugin termination failed
    #[error("Plugin termination failed: {0}")]
    TerminationFailed(String),

    /// Plugin is currently upgrading (hot reload in progress)
    #[error("Plugin is upgrading")]
    Upgrading,

    /// Plugin crashed or panicked
    #[error("Plugin crashed: {0}")]
    Crashed(String),

    /// Plugin was forcibly terminated
    #[error("Plugin was forcefully terminated: {0}")]
    ForciblyTerminated(String),
}

/// Errors related to capability operations.
#[derive(Debug, Error)]
pub enum CapabilityError {
    /// Capability with the given ID was not found
    #[error("Capability not found: {0}")]
    NotFound(CapabilityId),

    /// Capability has not been granted to the requester
    #[error("Capability not granted: {0}")]
    NotGranted(String),

    /// Permission denied due to capability restrictions
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Capability is invalid or malformed
    #[error("Invalid capability: {0}")]
    Invalid(String),

    /// Failed to revoke capability
    #[error("Capability revocation failed: {0}")]
    RevocationFailed(String),

    /// Failed to apply constraints to capability
    #[error("Capability constraint error: {0}")]
    ConstraintError(String),

    /// Failed to compose capabilities
    #[error("Capability composition error: {0}")]
    CompositionError(String),

    /// Failed to split capability
    #[error("Capability split error: {0}")]
    SplitError(String),

    /// Delegation of capability failed
    #[error("Capability delegation failed: {0}")]
    DelegationFailed(String),

    /// Capability is for the wrong resource type
    #[error("Capability type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected capability type
        expected: String,

        /// Actual capability type
        actual: String,
    },
}

/// Errors related to policy enforcement.
#[derive(Debug, Error)]
pub enum PolicyError {
    /// Policy rule with the given name was not found
    #[error("Policy rule not found: {0}")]
    RuleNotFound(String),

    /// Request violates policy rules
    #[error("Policy violation: {0}")]
    Violation(String),

    /// Failed to evaluate policy rules
    #[error("Policy evaluation failed: {0}")]
    EvaluationFailed(String),

    /// Network access violated policy rules
    #[error("Network access policy violation: {0}")]
    NetworkAccessViolation(String),

    /// File access violated policy rules
    #[error("File access policy violation: {0}")]
    FileAccessViolation(String),

    /// Resource usage exceeded policy limits
    #[error("Resource limit policy exceeded: {0}")]
    ResourceLimitExceeded(String),

    /// Policy could not be applied to capability
    #[error("Policy application failed: {0}")]
    ApplicationFailed(String),

    /// Policy conflict with existing policy
    #[error("Policy conflict: {0}")]
    Conflict(String),
}

/// Errors related to isolation operations.
#[derive(Debug, Error)]
pub enum IsolationError {
    /// Plugin has not been loaded
    #[error("Plugin not loaded: {0}")]
    PluginNotLoaded(PluginId),

    /// Failed to load plugin
    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    /// Failed to compile WebAssembly module
    #[error("Failed to compile module: {0}")]
    CompilationFailed(String),

    /// Failed to instantiate WebAssembly module
    #[error("Failed to instantiate module: {0}")]
    InstantiationFailed(String),

    /// Failed to link host functions to WebAssembly module
    #[error("Failed to link host functions: {0}")]
    LinkingFailed(String),

    /// WebAssembly execution trapped
    #[error("Execution trap: {0}")]
    ExecutionTrap(String),

    /// WebAssembly module format is invalid
    #[error("Invalid module format: {0}")]
    InvalidModuleFormat(String),

    /// Failed to access memory
    #[error("Memory access error: {0}")]
    MemoryAccessError(String),

    /// Memory region with the given ID was not found
    #[error("Memory region not found: {0}")]
    RegionNotFound(RegionId),

    /// Isolation boundary breach attempt detected
    #[error("Isolation breach attempt: {0}")]
    IsolationBreach(String),

    /// Module validation failed
    #[error("Module validation failed: {0}")]
    ValidationFailed(String),

    /// Resource limits exceeded during execution
    #[error("Resource limits exceeded: {0}")]
    ResourceExhausted(String),
}

/// Errors related to concurrency operations.
#[derive(Debug, Error)]
pub enum ConcurrencyError {
    /// Failed to create instance
    #[error("Failed to create instance: {0}")]
    InstanceCreationFailed(String),

    /// No available instances for the given plugin
    #[error("No available instances for plugin: {0}")]
    NoAvailableInstances(PluginId),

    /// Thread pool has been exhausted
    #[error("Thread pool exhausted")]
    ThreadPoolExhausted,

    /// Timed out waiting for an instance
    #[error("Acquisition timeout after {0}ms for plugin {1}")]
    AcquisitionTimeout(u64, PluginId),

    /// Instance pool size limit reached
    #[error("Instance pool limit reached: {0}")]
    PoolLimitReached(String),

    /// Failed to initialize actor
    #[error("Actor initialization failed: {0}")]
    ActorInitFailed(String),

    /// Failed to deliver message to actor
    #[error("Actor message delivery failed: {0}")]
    MessageDeliveryFailed(String),

    /// Supervisor error
    #[error("Supervisor error: {0}")]
    SupervisorError(String),

    /// Actor with the given ID was not found
    #[error("Actor not found: {0}")]
    ActorNotFound(String),

    /// Actor mailbox is full
    #[error("Actor mailbox full: {0}")]
    MailboxFull(String),

    /// Actor system is shutting down
    #[error("Actor system shutting down")]
    SystemShuttingDown,

    /// Actor crashed or panicked
    #[error("Actor crashed: {0}")]
    ActorCrashed(String),
}

/// Errors related to workflow operations.
#[derive(Debug, Error)]
pub enum WorkflowError {
    /// Workflow with the given ID was not found
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(WorkflowId),

    /// Node with the given ID was not found
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    /// Workflow definition is invalid
    #[error("Workflow definition error: {0}")]
    DefinitionError(String),

    /// Node execution failed
    #[error("Node execution failed: {0}")]
    NodeExecutionFailed(String),

    /// Workflow execution failed
    #[error("Workflow execution failed: {0}")]
    ExecutionFailed(String),

    /// Workflow execution timed out
    #[error("Workflow timeout after {0}ms")]
    Timeout(u64),

    /// Workflow execution was cancelled
    #[error("Workflow cancelled")]
    Cancelled,

    /// Workflow has a cyclic dependency
    #[error("Cyclic dependency detected")]
    CyclicDependency,

    /// Execution with the given ID was not found
    #[error("Execution not found: {0}")]
    ExecutionNotFound(ExecutionId),

    /// Failed to persist workflow state
    #[error("Persistence error: {0}")]
    PersistenceError(String),

    /// Maximum retry count reached for a node
    #[error("Max retry count reached for node: {0}")]
    MaxRetryCountReached(NodeId),

    /// Error dispatching workflow
    #[error("Workflow dispatch error: {0}")]
    DispatchError(String),

    /// Recovery from checkpoint failed
    #[error("Recovery from checkpoint failed: {0}")]
    RecoveryFailed(String),
}

/// Errors related to distribution operations.
#[derive(Debug, Error)]
pub enum DistributionError {
    /// Node with the given ID was not found
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Communication with remote node failed
    #[error("Communication failed: {0}")]
    CommunicationFailed(String),

    /// Failed to export capability
    #[error("Capability export failed: {0}")]
    CapabilityExportFailed(String),

    /// Failed to import capability
    #[error("Capability import failed: {0}")]
    CapabilityImportFailed(String),

    /// Failed to validate token
    #[error("Token validation failed: {0}")]
    TokenValidationFailed(String),

    /// Remote call failed
    #[error("Remote call failed: {0}")]
    RemoteCallFailed(String),

    /// Cluster membership error
    #[error("Cluster membership error: {0}")]
    MembershipError(String),

    /// Message with the given ID was not delivered
    #[error("Message not delivered: {0}")]
    MessageNotDelivered(MessageId),

    /// Network partition detected
    #[error("Network partition detected")]
    NetworkPartition,

    /// Message serialization or deserialization failed
    #[error("Message serialization error: {0}")]
    MessageSerializationError(String),

    /// Capability attestation signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),
}

/// Errors related to observability operations.
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// Failed to initialize tracing
    #[error("Tracing initialization failed: {0}")]
    TracingInitFailed(String),

    /// Failed to initialize metrics
    #[error("Metrics initialization failed: {0}")]
    MetricsInitFailed(String),

    /// Failed to initialize logging
    #[error("Logging initialization failed: {0}")]
    LoggingInitFailed(String),

    /// Failed to propagate context
    #[error("Context propagation failed: {0}")]
    ContextPropagationFailed(String),

    /// Failed to record metric
    #[error("Metric recording failed: {0}")]
    MetricRecordingFailed(String),

    /// Failed to export telemetry data
    #[error("Telemetry export failed: {0}")]
    TelemetryExportFailed(String),

    /// Failed to create span
    #[error("Span creation failed: {0}")]
    SpanCreationFailed(String),
}

/// Result type used throughout the Lion system.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        // Test conversion from PluginError to Error
        let plugin_err = PluginError::NotFound(PluginId::new());
        let error: Error = plugin_err.into();
        matches!(error, Error::Plugin(_));

        // Test conversion from CapabilityError to Error
        let cap_err = CapabilityError::NotFound(CapabilityId::new());
        let error: Error = cap_err.into();
        matches!(error, Error::Capability(_));

        // Test conversion from WorkflowError to Error
        let wf_err = WorkflowError::WorkflowNotFound(WorkflowId::new());
        let error: Error = wf_err.into();
        matches!(error, Error::Workflow(_));
    }

    #[test]
    fn test_error_display() {
        let plugin_id = PluginId::new();
        let plugin_err = PluginError::NotFound(plugin_id);
        let error: Error = plugin_err.into();
        let display = format!("{}", error);
        assert!(display.contains(&format!("Plugin not found: {}", plugin_id)));
    }
}
