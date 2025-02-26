//! Workflow-related data types.
//! 
//! This module defines data structures for workflow definition, execution,
//! nodes, and status.

use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::id::{WorkflowId, NodeId, ExecutionId, PluginId};

/// A workflow is a directed acyclic graph of nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow ID.
    pub id: WorkflowId,
    
    /// Human-readable name.
    pub name: String,
    
    /// Description of what this workflow does.
    pub description: String,
    
    /// Nodes in the workflow, indexed by ID.
    pub nodes: HashMap<NodeId, WorkflowNode>,
    
    /// Edges connecting nodes (dependencies).
    pub edges: HashMap<NodeId, HashSet<NodeId>>,
    
    /// Entry point nodes (no dependencies).
    pub entry_nodes: Vec<NodeId>,
    
    /// When this workflow was created.
    pub created_at: DateTime<Utc>,
    
    /// When this workflow was last updated.
    pub updated_at: DateTime<Utc>,
    
    /// Workflow version.
    pub version: String,
}

/// Type of workflow node.
#[derive(Clone, Debug, Serialize, Deserialize)]
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
        /// The condition expression.
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub created_at: DateTime<Utc>,
    
    /// When this node was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Status of a node execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is waiting for dependencies.
    Pending,
    
    /// Node is ready to execute.
    Ready,
    
    /// Node is currently executing.
    Running {
        /// When the node started.
        started_at: DateTime<Utc>,
        
        /// Number of retry attempts.
        retry_count: usize,
    },
    
    /// Node has completed successfully.
    Completed {
        /// When the node completed.
        completed_at: DateTime<Utc>,
        
        /// How long the node took to execute.
        duration_ms: u64,
        
        /// The node's results.
        results: serde_json::Value,
    },
    
    /// Node has failed.
    Failed {
        /// When the node failed.
        failed_at: DateTime<Utc>,
        
        /// How long the node took to execute.
        duration_ms: u64,
        
        /// The error message.
        error: String,
        
        /// Number of retry attempts.
        retry_count: usize,
    },
    
    /// Node was skipped due to condition.
    Skipped,
    
    /// Node was cancelled.
    Cancelled,
}

/// Status of a workflow execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Workflow is pending execution.
    Pending,
    
    /// Workflow is currently executing.
    Running {
        /// When the workflow started.
        started_at: DateTime<Utc>,
    },
    
    /// Workflow has completed successfully.
    Completed {
        /// When the workflow completed.
        completed_at: DateTime<Utc>,
        
        /// How long the workflow took to execute.
        duration_ms: u64,
        
        /// The workflow's results.
        results: serde_json::Value,
    },
    
    /// Workflow has failed.
    Failed {
        /// When the workflow failed.
        failed_at: DateTime<Utc>,
        
        /// How long the workflow took to execute.
        duration_ms: u64,
        
        /// The error message.
        error: String,
    },
    
    /// Workflow was cancelled.
    Cancelled,
}

/// Options for workflow execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// Maximum parallel nodes.
    pub max_parallel_nodes: usize,
    
    /// Timeout for the entire workflow, in milliseconds.
    pub timeout_ms: Option<u64>,
    
    /// Whether to continue execution if a node fails.
    pub continue_on_failure: bool,
    
    /// Whether to use checkpoints for persistence.
    pub use_checkpoints: bool,
    
    /// Checkpoint interval, in milliseconds.
    pub checkpoint_interval_ms: Option<u64>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_parallel_nodes: 4,
            timeout_ms: Some(300000), // 5 minutes
            continue_on_failure: false,
            use_checkpoints: false,
            checkpoint_interval_ms: Some(60000), // 1 minute
        }
    }
}