use std::any::Any;
use std::fmt::{Debug, Display};

use lion_core::error::ErrorCode;
use lion_core::id::CapabilityId;
use thiserror::Error;

/// Errors that can occur during capability operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Invalid constraint: {0}")]
    InvalidConstraint(String),

    #[error("Incompatible capability types for operation: {0}")]
    IncompatibleTypes(String),

    #[error("Operation not supported: {0}")]
    UnsupportedOperation(String),

    #[error("Capability not found: {0}")]
    NotFound(String),

    #[error("Internal capability system error: {0}")]
    InternalError(String),

    #[error("Invalid capability state: {0}")]
    InvalidState(String),

    #[error("Core error: {0}")]
    CoreError(#[from] lion_core::error::Error),
}

impl CapabilityError {
    pub fn code(&self) -> ErrorCode {
        match self {
            CapabilityError::AccessDenied(_) => ErrorCode::PermissionDenied,
            CapabilityError::InvalidConstraint(_) => ErrorCode::InvalidArgument,
            CapabilityError::IncompatibleTypes(_) => ErrorCode::InvalidArgument,
            CapabilityError::UnsupportedOperation(_) => ErrorCode::Unsupported,
            CapabilityError::NotFound(_) => ErrorCode::NotFound,
            CapabilityError::InternalError(_) => ErrorCode::Internal,
            CapabilityError::InvalidState(_) => ErrorCode::InvalidState,
            CapabilityError::CoreError(e) => e.code(),
        }
    }
}

/// Generic constraining mechanisms for capabilities
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Path-based constraint for filesystem capabilities
    FilePath(String),

    /// Operation-based constraint for file operations
    FileOperation {
        read: bool,
        write: bool,
        execute: bool,
    },

    /// Host-based constraint for network capabilities
    NetworkHost(String),

    /// Port-based constraint for network capabilities
    NetworkPort(u16),

    /// Operation-based constraint for network operations
    NetworkOperation {
        connect: bool,
        listen: bool,
        bind: bool,
    },

    /// Memory range constraint
    MemoryRange {
        base: usize,
        size: usize,
        read: bool,
        write: bool,
    },

    /// Plugin call constraint
    PluginCall { plugin_id: String, function: String },

    /// Message topic constraint
    MessageTopic(String),

    /// Custom constraint with type and value
    Custom {
        constraint_type: String,
        value: String,
    },
}

/// Access request type for checking capabilities
#[derive(Debug, Clone, PartialEq)]
pub enum AccessRequest {
    /// File access request
    File {
        path: String,
        read: bool,
        write: bool,
        execute: bool,
    },

    /// Network access request
    Network {
        host: Option<String>,
        port: Option<u16>,
        connect: bool,
        listen: bool,
        bind: bool,
    },

    /// Memory access request
    Memory {
        address: usize,
        size: usize,
        read: bool,
        write: bool,
    },

    /// Plugin call request
    PluginCall { plugin_id: String, function: String },

    /// Message sending request
    Message {
        topic: String,
        recipient: Option<String>,
    },

    /// Custom access request
    Custom {
        request_type: String,
        details: String,
    },
}

/// Core capability trait that defines the behavior of all capability types
///
/// Capabilities form a partial order based on privilege inclusion:
/// - A capability C₁ is less than or equal to another capability C₂ if C₁'s
///   permissions are a subset of C₂'s permissions.
/// - The "meet" operation represents the intersection of permissions.
/// - The "join" operation represents the union of permissions.
pub trait Capability: Send + Sync + Debug {
    /// Returns the type identifier for this capability
    fn capability_type(&self) -> &str;

    /// Checks if this capability permits the given access request
    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError>;

    /// Applies constraints to produce a more restricted capability
    fn constrain(&self, constraints: &[Constraint])
        -> Result<Box<dyn Capability>, CapabilityError>;

    /// Splits this capability into multiple smaller capabilities if possible
    fn split(&self) -> Vec<Box<dyn Capability>>;

    /// Combines this capability with another, if they are compatible
    /// Returns a new capability that permits what both input capabilities permit
    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError>;

    /// Returns true if this capability is less than or equal to another in the partial order
    /// (i.e., this grants a subset of the permissions that other grants)
    fn leq(&self, other: &dyn Capability) -> bool {
        // Default implementation: A ≤ B if for all requests r, if A permits r then B permits r
        // This is a conservative approach and should be overridden by specific capability types
        // for better performance
        false
    }

    /// Meet operation (intersection of rights) - provides the greatest lower bound
    /// in the capability partial order
    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        Err(CapabilityError::UnsupportedOperation(
            "Meet operation not implemented for this capability type".to_string(),
        ))
    }

    /// Clones this capability (since dyn Trait cannot implement Clone directly)
    fn clone_box(&self) -> Box<dyn Capability>;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Convert to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any {
        // Default implementation for capabilities that don't need mutable downcasting
        panic!("as_any_mut not implemented for this capability type")
    }
}

// Allow cloning of Box<dyn Capability>
impl Clone for Box<dyn Capability> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A utility for creating more granular capabilities by applying filters
pub struct CapabilityBuilder {
    base: Box<dyn Capability>,
    constraints: Vec<Constraint>,
}

impl CapabilityBuilder {
    /// Creates a new builder from a base capability
    pub fn new(base: Box<dyn Capability>) -> Self {
        Self {
            base,
            constraints: Vec::new(),
        }
    }

    /// Adds a constraint to further restrict the capability
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Builds the final capability by applying all constraints
    pub fn build(self) -> Result<Box<dyn Capability>, CapabilityError> {
        self.base.constrain(&self.constraints)
    }
}

/// Capability manager identifier for tracking which component a capability belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityOwner {
    /// Capability owned by a plugin
    Plugin(lion_core::id::PluginId),

    /// Capability owned by a system component
    System(lion_core::id::ComponentId),

    /// Capability owned by the kernel itself
    Kernel,
}

impl Display for CapabilityOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityOwner::Plugin(id) => write!(f, "Plugin({})", id),
            CapabilityOwner::System(id) => write!(f, "System({})", id),
            CapabilityOwner::Kernel => write!(f, "Kernel"),
        }
    }
}

/// Utility function to check if a path matches a pattern
/// Handles wildcards, prefixes, and exact matches
pub fn path_matches(pattern: &str, path: &str) -> bool {
    // Handle prefix matches with trailing /*
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return path.starts_with(prefix);
    }

    // Handle exact pattern matches
    pattern == path
}
