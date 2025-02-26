//! Core types for the Lion runtime.
//! 
//! This module defines the fundamental data structures shared across all Lion crates.
//! It aims to be minimal and focused, with no complex dependencies.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for a plugin.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub Uuid);

impl PluginId {
    /// Create a new random plugin ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    /// Create the system plugin ID (all zeros).
    pub fn system() -> Self {
        Self(Uuid::nil())
    }
    
    /// Check if this is the system plugin ID.
    pub fn is_system(&self) -> bool {
        self.0 == Uuid::nil()
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message identifier for inter-plugin communication.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    /// Create a new random message ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Shared memory region identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionId(pub Uuid);

impl RegionId {
    /// Create a new random region ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Plugin state in the lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is created but not yet initialized.
    Created,
    
    /// Plugin is initialized and ready to run.
    Ready,
    
    /// Plugin is actively running.
    Running,
    
    /// Plugin is paused.
    Paused,
    
    /// Plugin has failed.
    Failed,
    
    /// Plugin has been terminated.
    Terminated,
    
    /// Plugin is upgrading (hot reload).
    Upgrading,
}

/// Minimal message for inter-plugin communication.
/// 
/// This contains only the essential fields. Extended messaging
/// functionality is available in the messaging module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    /// Message identifier.
    pub id: MessageId,
    
    /// Sender plugin ID.
    pub sender: PluginId,
    
    /// Recipient plugin ID.
    pub recipient: PluginId,
    
    /// Binary message payload.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    
    /// Time-to-live in milliseconds (0 = unlimited)
    pub ttl_ms: u64,
    
    /// When the message was created.
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a new message with the given sender, recipient, and payload.
    pub fn new(sender: PluginId, recipient: PluginId, payload: Vec<u8>) -> Self {
        Self {
            id: MessageId::new(),
            sender,
            recipient,
            payload,
            ttl_ms: 0,
            created_at: Utc::now(),
        }
    }
    
    /// Set message time-to-live.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl_ms = ttl.as_millis() as u64;
        self
    }
    
    /// Check if the message has expired.
    pub fn is_expired(&self) -> bool {
        if self.ttl_ms == 0 {
            return false;
        }
        
        let age = Utc::now().signed_duration_since(self.created_at);
        age.num_milliseconds() > self.ttl_ms as i64
    }
}

/// Resource usage statistics for a plugin.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in bytes.
    pub memory_bytes: usize,
    
    /// CPU time used in microseconds.
    pub cpu_time_us: u64,
    
    /// Number of function calls executed.
    pub function_calls: u64,
    
    /// When resource usage was last updated.
    pub last_updated: DateTime<Utc>,
}

/// Plugin type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginType {
    /// WebAssembly plugin.
    Wasm,
    
    /// Native plugin (shared library).
    Native,
    
    /// JavaScript plugin.
    JavaScript,
    
    /// Remote plugin (in another process or node).
    Remote,
}

/// Basic plugin metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin ID.
    pub id: PluginId,
    
    /// Human-readable name.
    pub name: String,
    
    /// Version string.
    pub version: String,
    
    /// Description of the plugin.
    pub description: String,
    
    /// Plugin type.
    pub plugin_type: PluginType,
    
    /// Current state.
    pub state: PluginState,
    
    /// When the plugin was created.
    pub created_at: DateTime<Utc>,
    
    /// When the plugin was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Function execution result.
#[derive(Debug)]
pub enum FunctionResult {
    /// Success with binary data.
    Success(Vec<u8>),
    
    /// Error with message.
    Error(String),
}

/// Plugin configuration with resource limits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Maximum memory usage in bytes.
    pub max_memory_bytes: Option<usize>,
    
    /// Maximum CPU time in microseconds.
    pub max_cpu_time_us: Option<u64>,
    
    /// Function call timeout in milliseconds.
    pub function_timeout_ms: Option<u64>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(100 * 1024 * 1024), // 100 MB
            max_cpu_time_us: Some(10 * 1000 * 1000),   // 10 seconds
            function_timeout_ms: Some(5000),           // 5 seconds
        }
    }
}