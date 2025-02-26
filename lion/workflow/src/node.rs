//! Workflow nodes and node types.
//!
//! This module defines the types of nodes that can be used in workflows,
//! including function calls, conditionals, and subflows.

use std::time::Duration;
use core::error::{Result, WorkflowError};
use core::types::PluginId;
use crate::{NodeId, WorkflowId};

/// Type of workflow node.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    /// Call a plugin function.
    PluginCall {
        /// The plugin ID.
        plugin_id: PluginId,
        
        /// The function name.
        function: String,
        
        /// Input mapping from context to parameters.
        input_mapping: serde_json::Map<String, serde_json::Value>,
        
        /// Output mapping from results to context.
        output_mapping: serde_json::Map<String, serde_json::Value>,
    },
    
    /// Execute a conditional branch.
    Condition {
        /// The condition expression (JMESPath).
        condition: String,
        
        /// The node to execute if the condition is true.
        true_branch: NodeId,
        
        /// The node to execute if the condition is false.
        false_branch: NodeId,
    },
    
    /// Execute a subflow.
    Subflow {
        /// The workflow ID.
        workflow_id: WorkflowId,
        
        /// Input mapping from context to parameters.
        input_mapping: serde_json::Map<String, serde_json::Value>,
        
        /// Output mapping from results to context.
        output_mapping: serde_json::Map<String, serde_json::Value>,
    },
    
    /// Custom node type.
    Custom {
        /// The type name.
        type_name: String,
        
        /// Custom parameters.
        params: serde_json::Map<String, serde_json::Value>,
    },
}

/// Policy for handling errors.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ErrorPolicy {
    /// Fail the workflow on error.
    Fail,
    
    /// Ignore the error and continue.
    Continue,
    
    /// Retry the node.
    Retry {
        /// Maximum number of retries.
        max_retries: usize,
        
        /// Delay between retries, in milliseconds.
        retry_delay_ms: u64,
        
        /// Whether to use exponential backoff.
        exponential_backoff: bool,
    },
}

/// Configuration for a workflow node.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeConfig {
    /// Error handling policy.
    pub error_policy: ErrorPolicy,
    
    /// Timeout for the node, in milliseconds.
    pub timeout_ms: Option<u64>,
    
    /// Maximum memory usage, in bytes.
    pub max_memory_bytes: Option<usize>,
    
    /// Additional configuration.
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            error_policy: ErrorPolicy::Fail,
            timeout_ms: Some(30000), // 30 seconds
            max_memory_bytes: None,
            extra: serde_json::Map::new(),
        }
    }
}

/// A node in a workflow.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkflowNode {
    /// Unique node ID.
    pub id: NodeId,
    
    /// Human-readable name.
    pub name: String,
    
    /// Description of what this node does.
    pub description: String,
    
    /// Node type.
    pub node_type: NodeType,
    
    /// Node configuration.
    pub config: NodeConfig,
    
    /// When this node was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// When this node was last updated.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl WorkflowNode {
    /// Create a new workflow node.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        node_type: NodeType,
    ) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            id: NodeId::new(),
            name: name.into(),
            description: description.into(),
            node_type,
            config: NodeConfig::default(),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Set the node configuration.
    pub fn with_config(mut self, config: NodeConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Get the node timeout.
    pub fn timeout(&self) -> Option<Duration> {
        self.config.timeout_ms.map(|ms| Duration::from_millis(ms))
    }
    
    /// Check if this node has dependencies on the results of another node.
    pub fn depends_on_results(&self, node_id: &NodeId) -> bool {
        match &self.node_type {
            NodeType::PluginCall { input_mapping, .. } => {
                // Check if any input mapping references the node
                for value in input_mapping.values() {
                    if let serde_json::Value::String(s) = value {
                        if s.contains(&format!("$.nodes.{}", node_id)) {
                            return true;
                        }
                    }
                }
                false
            },
            NodeType::Condition { condition, .. } => {
                // Check if the condition references the node
                condition.contains(&format!("$.nodes.{}", node_id))
            },
            NodeType::Subflow { input_mapping, .. } => {
                // Check if any input mapping references the node
                for value in input_mapping.values() {
                    if let serde_json::Value::String(s) = value {
                        if s.contains(&format!("$.nodes.{}", node_id)) {
                            return true;
                        }
                    }
                }
                false
            },
            NodeType::Custom { params, .. } => {
                // Check if any parameter references the node
                for value in params.values() {
                    if let serde_json::Value::String(s) = value {
                        if s.contains(&format!("$.nodes.{}", node_id)) {
                            return true;
                        }
                    }
                }
                false
            },
        }
    }
}