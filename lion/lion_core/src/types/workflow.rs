//! Workflow-related data types.
//! 
//! This module defines data structures for workflow definitions, execution,
//! and status tracking. These types are used throughout the system to
//! define, execute, and monitor workflows.
//! 
//! The workflow system is based on the "Advanced Workflow Composition in
//! Lion WebAssembly Plugin System" research.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::id::{WorkflowId, NodeId, PluginId, ExecutionId};

/// A workflow definition.
///
/// A workflow is a directed acyclic graph of nodes, where each node represents
/// a unit of work. Workflows are used to orchestrate complex multi-step processes
/// with dependency management, parallel execution, and error handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique identifier for this workflow.
    pub id: WorkflowId,
    
    /// Human-readable name.
    pub name: String,
    
    /// Description of the workflow.
    pub description: String,
    
    /// The nodes in this workflow.
    pub nodes: Vec<WorkflowNode>,
}

impl Workflow {
    /// Create a new workflow.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the workflow.
    /// * `description` - Description of the workflow.
    ///
    /// # Returns
    ///
    /// A new workflow with a unique ID and no nodes.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: WorkflowId::new(),
            name: name.into(),
            description: description.into(),
            nodes: Vec::new(),
        }
    }
    
    /// Add a node to the workflow.
    ///
    /// # Arguments
    ///
    /// * `node` - The node to add.
    ///
    /// # Returns
    ///
    /// A reference to the newly added node.
    pub fn add_node(&mut self, node: WorkflowNode) -> &WorkflowNode {
        self.nodes.push(node);
        self.nodes.last().unwrap()
    }
    
    /// Get a node by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to get.
    ///
    /// # Returns
    ///
    /// A reference to the node, or `None` if no node with the given ID exists.
    pub fn get_node(&self, id: &NodeId) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|n| &n.id == id)
    }
    
    /// Get a mutable reference to a node by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to get.
    ///
    /// # Returns
    ///
    /// A mutable reference to the node, or `None` if no node with the given ID exists.
    pub fn get_node_mut(&mut self, id: &NodeId) -> Option<&mut WorkflowNode> {
        self.nodes.iter_mut().find(|n| &n.id == id)
    }
    
    /// Remove a node from the workflow.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to remove.
    ///
    /// # Returns
    ///
    /// The removed node, or `None` if no node with the given ID exists.
    pub fn remove_node(&mut self, id: &NodeId) -> Option<WorkflowNode> {
        let index = self.nodes.iter().position(|n| &n.id == id)?;
        Some(self.nodes.remove(index))
    }
    
    /// Get the entry nodes of the workflow.
    ///
    /// Entry nodes are nodes that have no dependencies, i.e., they
    /// can be executed immediately without waiting for other nodes.
    ///
    /// # Returns
    ///
    /// A vector of references to the entry nodes.
    pub fn entry_nodes(&self) -> Vec<&WorkflowNode> {
        self.nodes.iter().filter(|n| n.dependencies.is_empty()).collect()
    }
    
    /// Get the nodes that depend on a given node.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the node to get dependents for.
    ///
    /// # Returns
    ///
    /// A vector of references to the nodes that depend on the given node.
    pub fn get_dependents(&self, id: &NodeId) -> Vec<&WorkflowNode> {
        self.nodes.iter().filter(|n| n.dependencies.contains(id)).collect()
    }
    
    /// Validate the workflow.
    ///
    /// This checks for invalid dependencies and cycles in the workflow graph.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workflow is valid.
    /// * `Err(String)` with an error message if the workflow is invalid.
    pub fn validate(&self) -> Result<(), String> {
        // Check for invalid dependencies
        for node in &self.nodes {
            for dep_id in &node.dependencies {
                if !self.nodes.iter().any(|n| &n.id == dep_id) {
                    return Err(format!("Node {} depends on non-existent node {}", node.id, dep_id));
                }
            }
        }
        
        // Check for cycles using a depth-first search
        let mut visited = HashMap::new();
        for node in &self.nodes {
            if !visited.contains_key(&node.id) {
                let result = Self::check_cycle(&node.id, self, &mut visited, &mut Vec::new());
                if let Err(cycle) = result {
                    let cycle_str = cycle.iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    return Err(format!("Cycle detected in workflow: {}", cycle_str));
                }
            }
        }
        
        Ok(())
    }
    
    // Helper method for cycle detection
    fn check_cycle(
        node_id: &NodeId,
        workflow: &Workflow,
        visited: &mut HashMap<NodeId, bool>,
        path: &mut Vec<NodeId>,
    ) -> Result<(), Vec<NodeId>> {
        visited.insert(*node_id, true);
        path.push(*node_id);
        
        let node = workflow.get_node(node_id).unwrap();
        for dep_id in &node.dependencies {
            if !visited.contains_key(dep_id) {
                if let Err(cycle) = Self::check_cycle(dep_id, workflow, visited, path) {
                    return Err(cycle);
                }
            } else if *visited.get(dep_id).unwrap() {
                // We found a cycle
                let cycle_start = path.iter().position(|id| id == dep_id).unwrap();
                let mut cycle = path[cycle_start..].to_vec();
                cycle.push(*dep_id);
                return Err(cycle);
            }
        }
        
        visited.insert(*node_id, false);
        path.pop();
        Ok(())
    }
}

/// A node in a workflow.
///
/// A node represents a unit of work in a workflow, such as a plugin function call
/// or a built-in operation. Nodes can depend on other nodes, forming a directed
/// acyclic graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowNode {
    /// Unique identifier for this node.
    pub id: NodeId,
    
    /// Human-readable name.
    pub name: String,
    
    /// Type of this node.
    pub node_type: NodeType,
    
    /// Dependencies that must complete before this node can execute.
    pub dependencies: Vec<NodeId>,
    
    /// Error handling policy.
    pub error_policy: ErrorPolicy,
}

impl WorkflowNode {
    /// Create a new workflow node.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the node.
    /// * `node_type` - Type of the node.
    ///
    /// # Returns
    ///
    /// A new workflow node with a unique ID, the specified type, and no dependencies.
    pub fn new(name: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            id: NodeId::new(),
            name: name.into(),
            node_type,
            dependencies: Vec::new(),
            error_policy: ErrorPolicy::Fail,
        }
    }
    
    /// Create a new plugin call node.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the node.
    /// * `plugin_id` - ID of the plugin to call.
    /// * `function` - Name of the function to call.
    ///
    /// # Returns
    ///
    /// A new workflow node with a unique ID, the specified plugin call type, and no dependencies.
    pub fn new_plugin_call(
        name: impl Into<String>,
        plugin_id: impl Into<String>,
        function: impl Into<String>,
    ) -> Self {
        Self::new(
            name,
            NodeType::PluginCall {
                plugin_id: plugin_id.into(),
                function: function.into(),
            },
        )
    }
    
    /// Create a new map node.
    ///
    /// A map node applies the same operation to multiple items in parallel.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the node.
    /// * `plugin_id` - ID of the plugin to call.
    /// * `function` - Name of the function to call.
    ///
    /// # Returns
    ///
    /// A new workflow node with a unique ID, the specified map type, and no dependencies.
    pub fn new_map(
        name: impl Into<String>,
        plugin_id: impl Into<String>,
        function: impl Into<String>,
    ) -> Self {
        Self::new(
            name,
            NodeType::Map {
                plugin_id: plugin_id.into(),
                function: function.into(),
            },
        )
    }
    
    /// Add a dependency to this node.
    ///
    /// # Arguments
    ///
    /// * `dependency` - The ID of the node that must complete before this node can execute.
    pub fn add_dependency(&mut self, dependency: NodeId) {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
        }
    }
    
    /// Set the error policy for this node.
    ///
    /// # Arguments
    ///
    /// * `policy` - The error policy to use for this node.
    pub fn set_error_policy(&mut self, policy: ErrorPolicy) {
        self.error_policy = policy;
    }
}

/// Type of workflow node.
///
/// This enum represents the different types of operations that a workflow
/// node can perform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// Call a function in a plugin.
    PluginCall {
        /// ID of the plugin to call.
        plugin_id: String,
        
        /// Name of the function to call.
        function: String,
    },
    
    /// Map a function over a collection of items.
    Map {
        /// ID of the plugin to call.
        plugin_id: String,
        
        /// Name of the function to call.
        function: String,
    },
    
    /// Merge the results of multiple nodes.
    Merge {
        /// Strategy to use when merging results.
        strategy: MergeStrategy,
    },
    
    /// Conditional branching based on a condition.
    Condition {
        /// ID of the plugin to call for the condition.
        plugin_id: String,
        
        /// Name of the function to call for the condition.
        function: String,
        
        /// Node to execute if the condition is true.
        true_branch: Box<NodeId>,
        
        /// Node to execute if the condition is false.
        false_branch: Box<NodeId>,
    },
    
    /// Custom node type, handled by a plugin.
    Custom {
        /// ID of the plugin handling this node.
        plugin_id: String,
        
        /// Type identifier for the custom node.
        type_id: String,
        
        /// Custom configuration for this node.
        config: serde_json::Value,
    },
}

/// Strategy for merging results from multiple nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Concatenate the results.
    Concat,
    
    /// Merge JSON objects.
    JsonMerge,
    
    /// Use a custom merge function.
    Custom {
        /// ID of the plugin to call.
        plugin_id: String,
        
        /// Name of the function to call.
        function: String,
    },
}

/// Error handling policy for workflow nodes.
///
/// This enum defines how errors in workflow nodes are handled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorPolicy {
    /// Fail the workflow immediately.
    Fail,
    
    /// Retry the node a specified number of times.
    Retry {
        /// Maximum number of retry attempts.
        max_attempts: u32,
    },
    
    /// Skip the node and continue with the workflow.
    Skip,
    
    /// Use a custom error handler.
    Custom {
        /// ID of the plugin to call.
        plugin_id: String,
        
        /// Name of the function to call.
        function: String,
    },
}

/// Status of a workflow execution.
///
/// This enum represents the current status of a workflow execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// The execution is pending (not yet started).
    Pending,
    
    /// The execution is running.
    Running,
    
    /// The execution has completed successfully.
    Completed,
    
    /// The execution has failed.
    Failed,
    
    /// The execution has been cancelled.
    Cancelled,
    
    /// The execution is paused.
    Paused,
}

/// Status of a workflow node.
///
/// This enum represents the current status of a node in a workflow execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// The node is pending (not yet started).
    Pending,
    
    /// The node is waiting for dependencies to complete.
    Waiting,
    
    /// The node is running.
    Running,
    
    /// The node has completed successfully.
    Completed,
    
    /// The node has failed.
    Failed,
    
    /// The node has been skipped.
    Skipped,
    
    /// The node is being retried after a failure.
    Retrying,
}

/// Options for workflow execution.
///
/// This structure contains options for executing a workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// Initial input data for the workflow.
    pub input: Option<serde_json::Value>,
    
    /// Maximum execution time in milliseconds.
    pub timeout_ms: Option<u64>,
    
    /// Maximum number of concurrent nodes to execute.
    pub max_concurrency: Option<usize>,
    
    /// Whether to enable checkpointing for this execution.
    pub enable_checkpointing: bool,
    
    /// Custom tags for this execution.
    pub tags: HashMap<String, String>,
    
    /// Callback URL to notify when the execution completes.
    pub callback_url: Option<String>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            input: None,
            timeout_ms: Some(300_000), // 5 minutes
            max_concurrency: Some(4),
            enable_checkpointing: false,
            tags: HashMap::new(),
            callback_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_workflow() -> Workflow {
        let mut workflow = Workflow::new("Test Workflow", "A test workflow");
        
        let node1 = WorkflowNode::new_plugin_call("Node 1", "plugin1", "function1");
        let node1_id = node1.id;
        workflow.add_node(node1);
        
        let node2 = WorkflowNode::new_plugin_call("Node 2", "plugin2", "function2");
        let node2_id = node2.id;
        workflow.add_node(node2);
        
        let mut node3 = WorkflowNode::new_plugin_call("Node 3", "plugin3", "function3");
        node3.add_dependency(node1_id);
        node3.add_dependency(node2_id);
        workflow.add_node(node3);
        
        workflow
    }
    
    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("Test Workflow", "A test workflow");
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.description, "A test workflow");
        assert!(workflow.nodes.is_empty());
    }
    
    #[test]
    fn test_add_get_remove_node() {
        let mut workflow = Workflow::new("Test Workflow", "A test workflow");
        
        // Add a node
        let node = WorkflowNode::new_plugin_call("Test Node", "plugin1", "function1");
        let node_id = node.id;
        workflow.add_node(node);
        
        // Get the node
        let retrieved = workflow.get_node(&node_id).unwrap();
        assert_eq!(retrieved.name, "Test Node");
        
        // Get a mutable reference to the node
        let retrieved_mut = workflow.get_node_mut(&node_id).unwrap();
        retrieved_mut.name = "Updated Node".to_string();
        
        // Check that the update was applied
        let retrieved = workflow.get_node(&node_id).unwrap();
        assert_eq!(retrieved.name, "Updated Node");
        
        // Remove the node
        let removed = workflow.remove_node(&node_id).unwrap();
        assert_eq!(removed.id, node_id);
        
        // Check that the node is no longer in the workflow
        assert!(workflow.get_node(&node_id).is_none());
    }
    
    #[test]
    fn test_dependencies() {
        let workflow = create_test_workflow();
        
        // Check entry nodes
        let entry_nodes = workflow.entry_nodes();
        assert_eq!(entry_nodes.len(), 2);
        assert!(entry_nodes.iter().any(|n| n.name == "Node 1"));
        assert!(entry_nodes.iter().any(|n| n.name == "Node 2"));
        
        // Check dependents
        let node1 = workflow.nodes.iter().find(|n| n.name == "Node 1").unwrap();
        let dependents = workflow.get_dependents(&node1.id);
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].name, "Node 3");
    }
    
    #[test]
    fn test_workflow_validation() {
        let mut workflow = create_test_workflow();
        
        // Valid workflow should validate successfully
        assert!(workflow.validate().is_ok());
        
        // Adding an invalid dependency should fail validation
        let node3 = workflow.nodes.iter_mut().find(|n| n.name == "Node 3").unwrap();
        node3.dependencies.push(NodeId::new());
        assert!(workflow.validate().is_err());
        
        // Create a workflow with a cycle
        let mut workflow = Workflow::new("Cycle Workflow", "A workflow with a cycle");
        
        let node1 = WorkflowNode::new_plugin_call("Node 1", "plugin1", "function1");
        let node1_id = node1.id;
        workflow.add_node(node1);
        
        let mut node2 = WorkflowNode::new_plugin_call("Node 2", "plugin2", "function2");
        node2.add_dependency(node1_id);
        let node2_id = node2.id;
        workflow.add_node(node2);
        
        let mut node3 = WorkflowNode::new_plugin_call("Node 3", "plugin3", "function3");
        node3.add_dependency(node2_id);
        let node3_id = node3.id;
        workflow.add_node(node3);
        
        // Create a cycle by making node1 depend on node3
        let node1 = workflow.get_node_mut(&node1_id).unwrap();
        node1.add_dependency(node3_id);
        
        // Validation should fail due to the cycle
        assert!(workflow.validate().is_err());
    }
    
    #[test]
    fn test_node_creation() {
        // Test basic node creation
        let node = WorkflowNode::new("Test Node", NodeType::PluginCall {
            plugin_id: "plugin1".to_string(),
            function: "function1".to_string(),
        });
        assert_eq!(node.name, "Test Node");
        assert!(node.dependencies.is_empty());
        assert_eq!(node.error_policy, ErrorPolicy::Fail);
        
        // Test plugin call node creation
        let node = WorkflowNode::new_plugin_call("Test Node", "plugin1", "function1");
        match node.node_type {
            NodeType::PluginCall { plugin_id, function } => {
                assert_eq!(plugin_id, "plugin1");
                assert_eq!(function, "function1");
            },
            _ => panic!("Expected NodeType::PluginCall"),
        }
        
        // Test map node creation
        let node = WorkflowNode::new_map("Test Node", "plugin1", "function1");
        match node.node_type {
            NodeType::Map { plugin_id, function } => {
                assert_eq!(plugin_id, "plugin1");
                assert_eq!(function, "function1");
            },
            _ => panic!("Expected NodeType::Map"),
        }
    }
    
    #[test]
    fn test_node_dependencies() {
        let mut node = WorkflowNode::new_plugin_call("Test Node", "plugin1", "function1");
        
        // Add a dependency
        let dep_id = NodeId::new();
        node.add_dependency(dep_id);
        assert_eq!(node.dependencies.len(), 1);
        assert_eq!(node.dependencies[0], dep_id);
        
        // Adding the same dependency again should not duplicate
        node.add_dependency(dep_id);
        assert_eq!(node.dependencies.len(), 1);
        
        // Add another dependency
        let dep_id2 = NodeId::new();
        node.add_dependency(dep_id2);
        assert_eq!(node.dependencies.len(), 2);
        assert_eq!(node.dependencies[1], dep_id2);
    }
    
    #[test]
    fn test_error_policy() {
        let mut node = WorkflowNode::new_plugin_call("Test Node", "plugin1", "function1");
        
        // Default error policy should be Fail
        assert_eq!(node.error_policy, ErrorPolicy::Fail);
        
        // Set a new error policy
        node.set_error_policy(ErrorPolicy::Retry { max_attempts: 3 });
        match node.error_policy {
            ErrorPolicy::Retry { max_attempts } => assert_eq!(max_attempts, 3),
            _ => panic!("Expected ErrorPolicy::Retry"),
        }
    }
    
    #[test]
    fn test_execution_options() {
        // Test default options
        let options = ExecutionOptions::default();
        assert_eq!(options.timeout_ms, Some(300_000));
        assert_eq!(options.max_concurrency, Some(4));
        assert!(!options.enable_checkpointing);
        
        // Test custom options
        let mut options = ExecutionOptions {
            input: Some(serde_json::json!({"key": "value"})),
            timeout_ms: Some(60_000),
            max_concurrency: Some(10),
            enable_checkpointing: true,
            tags: HashMap::new(),
            callback_url: Some("https://example.com/callback".to_string()),
        };
        
        // Add tags
        options.tags.insert("environment".to_string(), "production".to_string());
        
        assert_eq!(options.timeout_ms, Some(60_000));
        assert_eq!(options.max_concurrency, Some(10));
        assert!(options.enable_checkpointing);
        assert_eq!(options.tags.get("environment").unwrap(), "production");
        assert_eq!(options.callback_url, Some("https://example.com/callback".to_string()));
    }
    
    #[test]
    fn test_serialization() {
        let workflow = create_test_workflow();
        
        // Serialize the workflow
        let serialized = serde_json::to_string(&workflow).unwrap();
        
        // Deserialize the workflow
        let deserialized: Workflow = serde_json::from_str(&serialized).unwrap();
        
        // Check that the workflow was correctly serialized and deserialized
        assert_eq!(workflow.id, deserialized.id);
        assert_eq!(workflow.name, deserialized.name);
        assert_eq!(workflow.description, deserialized.description);
        assert_eq!(workflow.nodes.len(), deserialized.nodes.len());
        
        // Check that node dependencies were preserved
        let original_node3 = workflow.nodes.iter().find(|n| n.name == "Node 3").unwrap();
        let deserialized_node3 = deserialized.nodes.iter().find(|n| n.name == "Node 3").unwrap();
        assert_eq!(original_node3.dependencies.len(), deserialized_node3.dependencies.len());
        
        // Test ExecutionOptions serialization
        let options = ExecutionOptions {
            input: Some(serde_json::json!({"key": "value"})),
            timeout_ms: Some(60_000),
            max_concurrency: Some(10),
            enable_checkpointing: true,
            tags: {
                let mut map = HashMap::new();
                map.insert("environment".to_string(), "production".to_string());
                map
            },
            callback_url: Some("https://example.com/callback".to_string()),
        };
        
        let serialized = serde_json::to_string(&options).unwrap();
        let deserialized: ExecutionOptions = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(options.timeout_ms, deserialized.timeout_ms);
        assert_eq!(options.max_concurrency, deserialized.max_concurrency);
        assert_eq!(options.enable_checkpointing, deserialized.enable_checkpointing);
        assert_eq!(options.tags.get("environment").unwrap(), deserialized.tags.get("environment").unwrap());
        assert_eq!(options.callback_url, deserialized.callback_url);
    }
}