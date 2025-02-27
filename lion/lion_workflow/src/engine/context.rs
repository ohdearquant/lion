use crate::model::{EdgeId, NodeId, NodeStatus, WorkflowDefinition};
use crate::state::{ConditionResult, WorkflowState};
use lion_core::id::PluginId;
use lion_core::CapabilityId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

/// Trait for capability checking
pub trait CapabilityChecker: Send + Sync {
    /// Check if a subject has permission to perform an action on an object
    fn check_permission(
        &self,
        subject: &str,
        object: &str,
        action: &str,
    ) -> Result<PermissionResult, String>;
}

/// Result of a permission check
#[derive(Debug, Clone, Copy)]
pub struct PermissionResult(pub bool);

impl PermissionResult {
    pub fn is_allowed(&self) -> bool {
        self.0
    }
}

/// Error types for workflow execution context
#[derive(Error, Debug)]
pub enum ContextError {
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("Edge not found: {0}")]
    EdgeNotFound(EdgeId),

    #[error("Capability error: {0}")]
    CapabilityError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Node error: {0}")]
    NodeError(String),
}

/// Result of a node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    /// Node ID
    pub node_id: NodeId,

    /// Status after execution
    pub status: NodeStatus,

    /// Output data
    pub output: serde_json::Value,

    /// Error information (if failed)
    pub error: Option<serde_json::Value>,

    /// Execution metadata
    pub metadata: serde_json::Value,

    /// Execution timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl NodeResult {
    /// Create a new successful node result
    pub fn success(node_id: NodeId, output: serde_json::Value) -> Self {
        NodeResult {
            node_id,
            status: NodeStatus::Completed,
            output,
            error: None,
            metadata: serde_json::Value::Null,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new failed node result
    pub fn failure(node_id: NodeId, error: serde_json::Value) -> Self {
        NodeResult {
            node_id,
            status: NodeStatus::Failed,
            output: serde_json::Value::Null,
            error: Some(error),
            metadata: serde_json::Value::Null,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add metadata to this result
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if this result represents a successful execution
    pub fn is_success(&self) -> bool {
        self.status == NodeStatus::Completed
    }

    /// Check if this result represents a failed execution
    pub fn is_failure(&self) -> bool {
        self.status == NodeStatus::Failed
    }
}

/// Context for workflow execution
///
/// This context is passed to node handlers and provides access to
/// workflow state, node inputs, and capabilities.
#[derive(Clone)]
pub struct ExecutionContext {
    /// The workflow definition
    pub definition: Arc<WorkflowDefinition>,

    /// Current workflow state
    pub state: Arc<WorkflowState>,

    /// Capability checker for capability-based security
    pub capability_checker: Option<Arc<dyn CapabilityChecker + 'static>>,

    /// Currently executing node ID
    pub current_node_id: Option<NodeId>,

    /// Execution variables (shared across nodes)
    pub variables: HashMap<String, serde_json::Value>,

    /// Task priority (used for scheduling)
    pub priority: u8,

    /// Execution attempt number (for retries)
    pub attempt: u32,

    /// Execution deadline (if any)
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(definition: Arc<WorkflowDefinition>, state: Arc<WorkflowState>) -> Self {
        ExecutionContext {
            definition,
            state,
            capability_checker: None,
            current_node_id: None,
            variables: HashMap::new(),
            priority: 1,
            attempt: 1,
            deadline: None,
        }
    }

    /// Set the capability checker
    pub fn with_capability_checker(
        mut self,
        checker: Arc<dyn CapabilityChecker + 'static>,
    ) -> Self {
        self.capability_checker = Some(checker);
        self
    }

    /// Set the current node ID
    pub fn with_node(mut self, node_id: NodeId) -> Self {
        self.current_node_id = Some(node_id);
        self
    }

    /// Set the task priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the execution attempt
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    /// Set the execution deadline
    pub fn with_deadline(mut self, deadline: chrono::DateTime<chrono::Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Get the current node
    pub fn get_current_node(&self) -> Result<&crate::model::Node, ContextError> {
        let node_id = self
            .current_node_id
            .ok_or_else(|| ContextError::InvalidState("No current node".to_string()))?;

        self.definition
            .get_node(&node_id)
            .ok_or_else(|| ContextError::NodeNotFound(node_id))
    }

    /// Get input data for the current node
    pub fn get_inputs(&self) -> Result<HashMap<NodeId, serde_json::Value>, ContextError> {
        let node_id = self
            .current_node_id
            .ok_or_else(|| ContextError::InvalidState("No current node".to_string()))?;

        // Get parent nodes
        let parent_nodes = self
            .definition
            .get_parent_nodes(&node_id)
            .map_err(|e| ContextError::ExecutionError(e.to_string()))?;

        // Collect results from parent nodes
        let mut inputs = HashMap::new();
        for parent in parent_nodes {
            if let Some(result) = self.state.node_results.get(&parent.id) {
                inputs.insert(parent.id, result.clone());
            }
        }

        Ok(inputs)
    }

    /// Check if the current context has a required capability
    pub fn has_capability(&self, capability_id: &CapabilityId) -> Result<bool, ContextError> {
        // If no checker is provided, assume all capabilities are allowed
        if self.capability_checker.is_none() {
            return Ok(true);
        }

        let checker = self.capability_checker.as_ref().unwrap();

        // TODO: Get the actual subject and action from the context
        let subject = "workflow_executor"; // Placeholder
        let object = capability_id.to_string();
        let action = "execute"; // Placeholder

        checker
            .check_permission(subject, &object, action)
            .map_err(|e| ContextError::CapabilityError(e))
            .map(|result| result.is_allowed())
    }

    /// Set a variable in the context
    pub fn set_variable(&mut self, name: &str, value: serde_json::Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Get a variable from the context
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    /// Evaluate an edge condition
    pub fn evaluate_condition(&self, edge_id: &EdgeId) -> Result<ConditionResult, ContextError> {
        let edge = self
            .definition
            .get_edge(edge_id)
            .ok_or_else(|| ContextError::EdgeNotFound(*edge_id))?;

        // Get source node result
        let source_result = self.state.node_results.get(&edge.source).ok_or_else(|| {
            ContextError::InvalidState(format!("No result for source node {}", edge.source))
        })?;

        match &edge.condition {
            crate::model::ConditionType::None => {
                // No condition, always passes
                Ok(ConditionResult::Passed)
            }
            crate::model::ConditionType::JsonPath(path) => {
                // Evaluate JSON path against source node result
                match evaluate_json_path(source_result, path) {
                    Ok(true) => Ok(ConditionResult::Passed),
                    Ok(false) => Ok(ConditionResult::Failed),
                    Err(_) => Ok(ConditionResult::Error),
                }
            }
            crate::model::ConditionType::Expression(expr) => {
                // Evaluate expression against context
                match evaluate_expression(expr, source_result, &self.variables) {
                    Ok(true) => Ok(ConditionResult::Passed),
                    Ok(false) => Ok(ConditionResult::Failed),
                    Err(_) => Ok(ConditionResult::Error),
                }
            }
            crate::model::ConditionType::Custom { plugin_id, config } => {
                // Custom condition, not implemented in this context
                // Would require plugin system integration
                Ok(ConditionResult::Pending)
            }
        }
    }
}

impl fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("current_node_id", &self.current_node_id)
            .field("priority", &self.priority)
            .field("attempt", &self.attempt)
            .field("deadline", &self.deadline)
            .field("variables", &self.variables)
            .finish()
    }
}

/// Evaluate a JSON path against a value
fn evaluate_json_path(value: &serde_json::Value, path: &str) -> Result<bool, String> {
    // Very simple implementation - would use a proper JSON path library in production
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for segment in segments {
        if segment.starts_with('[') && segment.ends_with(']') {
            // Array index
            let index = segment[1..segment.len() - 1]
                .parse::<usize>()
                .map_err(|_| format!("Invalid array index: {}", segment))?;

            current = current
                .get(index)
                .ok_or_else(|| format!("Index out of bounds: {}", index))?;
        } else {
            // Object field
            current = current
                .get(segment)
                .ok_or_else(|| format!("Field not found: {}", segment))?;
        }
    }

    // Convert final value to boolean
    match current {
        serde_json::Value::Bool(b) => Ok(*b),
        serde_json::Value::Number(n) => Ok(!n.is_i64() || n.as_i64().unwrap() != 0),
        serde_json::Value::String(s) => Ok(!s.is_empty()),
        serde_json::Value::Array(a) => Ok(!a.is_empty()),
        serde_json::Value::Object(o) => Ok(!o.is_empty()),
        serde_json::Value::Null => Ok(false),
    }
}

/// Evaluate an expression against a context
fn evaluate_expression(
    expr: &str,
    source_result: &serde_json::Value,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<bool, String> {
    // Very simple expression evaluator - would use a proper expression engine in production
    // This just checks if the expression is a direct boolean value
    match expr {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("Unsupported expression: {}", expr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, Node};

    // Helper to create a test workflow
    fn create_test_workflow() -> (Arc<WorkflowDefinition>, Arc<WorkflowState>) {
        let mut workflow =
            WorkflowDefinition::new(crate::model::WorkflowId::new(), "Test Workflow".to_string());

        let node1 = Node::new(NodeId::new(), "Node 1".to_string());
        let node2 = Node::new(NodeId::new(), "Node 2".to_string());
        let node1_id = node1.id;
        let node2_id = node2.id;

        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        workflow
            .add_edge(Edge::new(EdgeId::new(), node1_id, node2_id))
            .unwrap();

        let definition = Arc::new(workflow);
        let state = Arc::new(WorkflowState::new(definition.clone()));

        (definition, state)
    }

    #[test]
    fn test_node_result() {
        let node_id = NodeId::new();

        // Test success
        let success = NodeResult::success(node_id, serde_json::json!({"data": "test"}));
        assert!(success.is_success());
        assert!(!success.is_failure());
        assert_eq!(success.status, NodeStatus::Completed);

        // Test failure
        let failure = NodeResult::failure(node_id, serde_json::json!({"error": "test error"}));
        assert!(!failure.is_success());
        assert!(failure.is_failure());
        assert_eq!(failure.status, NodeStatus::Failed);
    }

    #[test]
    fn test_execution_context() {
        let (definition, state) = create_test_workflow();
        let node_id = *definition.nodes.keys().next().unwrap();

        let context = ExecutionContext::new(definition, state)
            .with_node(node_id)
            .with_priority(2)
            .with_attempt(3);

        assert_eq!(context.current_node_id, Some(node_id));
        assert_eq!(context.priority, 2);
        assert_eq!(context.attempt, 3);

        // Get current node
        let node = context.get_current_node().unwrap();
        assert_eq!(node.id, node_id);
    }

    #[test]
    fn test_json_path_evaluation() {
        let value = serde_json::json!({
            "a": {
                "b": {
                    "c": true
                }
            },
            "array": [1, 2, 3]
        });

        // Test success cases
        assert_eq!(evaluate_json_path(&value, "a.b.c").unwrap(), true);
        assert_eq!(evaluate_json_path(&value, "array[0]").unwrap(), true);

        // Test failure cases
        assert!(evaluate_json_path(&value, "non_existent").is_err());
        assert!(evaluate_json_path(&value, "array[10]").is_err());
    }
}
