use crate::model::edge::EdgeId;
use lion_core::id::Id;
use lion_core::CapabilityId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Unique identifier for workflow nodes
pub type NodeId = Id<Node>;

/// Status of a workflow node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
pub enum NodeStatus {
    #[default]
    /// Node is waiting for dependencies to complete
    Pending,
    /// Node is ready for execution (all dependencies met)
    Ready,
    /// Node is currently executing
    Running,
    /// Node has completed successfully
    Completed,
    /// Node has failed execution
    Failed,
    /// Node has been skipped (e.g., conditional branch)
    Skipped,
    /// Node execution has been cancelled
    Cancelled,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Pending => write!(f, "Pending"),
            NodeStatus::Ready => write!(f, "Ready"),
            NodeStatus::Running => write!(f, "Running"),
            NodeStatus::Completed => write!(f, "Completed"),
            NodeStatus::Failed => write!(f, "Failed"),
            NodeStatus::Skipped => write!(f, "Skipped"),
            NodeStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Priority level for node execution
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord, Hash, Default,
)]
pub enum Priority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// A node in the workflow graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    /// Unique identifier for this node
    pub id: NodeId,

    /// Human-readable name of the node
    pub name: String,

    /// Current status of this node
    #[serde(skip)]
    pub status: NodeStatus,

    /// Number of incomplete dependencies (in-degree)
    #[serde(skip)]
    pub in_degree: usize,

    /// IDs of outgoing edges (to child nodes)
    pub outgoing_edges: HashSet<EdgeId>,

    /// IDs of incoming edges (from parent nodes)
    pub incoming_edges: HashSet<EdgeId>,

    /// Capability required to execute this node
    pub required_capability: Option<CapabilityId>,

    /// Execution priority of this node
    pub priority: Priority,

    /// Optional deadline for node execution
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,

    /// Type-specific configuration for this node
    pub config: serde_json::Value,
}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Only hash fields that implement Hash
        // Skip HashSet fields (outgoing_edges and incoming_edges)
        self.id.hash(state);
        self.name.hash(state);
        self.status.hash(state);
        self.in_degree.hash(state);
        self.required_capability.hash(state);
        self.priority.hash(state);
        // Skip deadline as chrono::DateTime doesn't implement Hash
        // Skip config as serde_json::Value doesn't implement Hash
    }
}

impl Node {
    /// Create a new workflow node
    pub fn new(id: NodeId, name: String) -> Self {
        Node {
            id,
            name,
            status: NodeStatus::Pending,
            in_degree: 0,
            outgoing_edges: HashSet::new(),
            incoming_edges: HashSet::new(),
            required_capability: None,
            priority: Priority::Normal,
            deadline: None,
            config: serde_json::Value::Null,
        }
    }

    /// Check if this node is ready to execute (all dependencies completed)
    pub fn is_ready(&self) -> bool {
        self.status == NodeStatus::Pending && self.in_degree == 0
    }

    /// Mark this node as ready for execution
    pub fn mark_ready(&mut self) {
        if self.status == NodeStatus::Pending && self.in_degree == 0 {
            self.status = NodeStatus::Ready;
        }
    }

    /// Mark this node as running
    pub fn mark_running(&mut self) {
        if self.status == NodeStatus::Ready {
            self.status = NodeStatus::Running;
        }
    }

    /// Mark this node as completed
    pub fn mark_completed(&mut self) {
        self.status = NodeStatus::Completed;
    }

    /// Mark this node as failed
    pub fn mark_failed(&mut self) {
        self.status = NodeStatus::Failed;
    }

    /// Add a required capability to this node
    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.required_capability = Some(capability_id);
        self
    }

    /// Set the priority for this node
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set a deadline for this node
    pub fn with_deadline(mut self, deadline: chrono::DateTime<chrono::Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Set configuration for this node
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }

    /// Increment the in-degree counter for this node
    pub fn increment_in_degree(&mut self) {
        self.in_degree += 1;
    }

    /// Decrement the in-degree counter for this node
    /// Returns true if the node becomes ready (in_degree == 0)
    pub fn decrement_in_degree(&mut self) -> bool {
        if self.in_degree > 0 {
            self.in_degree -= 1;
            if self.in_degree == 0 && self.status == NodeStatus::Pending {
                self.status = NodeStatus::Ready;
                return true;
            }
        }
        false
    }

    /// Add an outgoing edge to this node
    pub fn add_outgoing_edge(&mut self, edge_id: EdgeId) {
        self.outgoing_edges.insert(edge_id);
    }

    /// Add an incoming edge to this node
    pub fn add_incoming_edge(&mut self, edge_id: EdgeId) {
        self.incoming_edges.insert(edge_id);
        self.increment_in_degree();
    }

    /// Remove an outgoing edge from this node
    pub fn remove_outgoing_edge(&mut self, edge_id: &EdgeId) -> bool {
        self.outgoing_edges.remove(edge_id)
    }

    /// Remove an incoming edge from this node
    pub fn remove_incoming_edge(&mut self, edge_id: &EdgeId) -> bool {
        if self.incoming_edges.remove(edge_id) {
            self.decrement_in_degree();
            return true;
        }
        false
    }
}

/// Thread-safe in-degree counter for concurrent workflow execution
pub struct AtomicNode {
    /// Node identifier
    pub id: NodeId,

    /// Atomic counter for in-degree (number of incomplete dependencies)
    in_degree: AtomicUsize,

    /// Current node status
    status: std::sync::atomic::AtomicU8,
}

impl AtomicNode {
    /// Create a new atomic node from a regular node
    pub fn new(node: &Node) -> Self {
        let status_value = match node.status {
            NodeStatus::Pending => 0,
            NodeStatus::Ready => 1,
            NodeStatus::Running => 2,
            NodeStatus::Completed => 3,
            NodeStatus::Failed => 4,
            NodeStatus::Skipped => 5,
            NodeStatus::Cancelled => 6,
        };

        AtomicNode {
            id: node.id.clone(),
            in_degree: AtomicUsize::new(node.in_degree),
            status: std::sync::atomic::AtomicU8::new(status_value),
        }
    }

    /// Atomically decrement the in-degree and check if the node becomes ready
    pub fn decrement_in_degree(&self) -> bool {
        let previous = self.in_degree.fetch_sub(1, Ordering::SeqCst);
        if previous == 1 && self.get_status() == NodeStatus::Pending {
            self.set_status(NodeStatus::Ready);
            return true;
        }
        false
    }

    /// Get the current status of this node
    pub fn get_status(&self) -> NodeStatus {
        match self.status.load(Ordering::SeqCst) {
            0 => NodeStatus::Pending,
            1 => NodeStatus::Ready,
            2 => NodeStatus::Running,
            3 => NodeStatus::Completed,
            4 => NodeStatus::Failed,
            5 => NodeStatus::Skipped,
            6 => NodeStatus::Cancelled,
            _ => NodeStatus::Pending, // Default fallback
        }
    }

    /// Set the status of this node
    pub fn set_status(&self, status: NodeStatus) {
        let value = match status {
            NodeStatus::Pending => 0,
            NodeStatus::Ready => 1,
            NodeStatus::Running => 2,
            NodeStatus::Completed => 3,
            NodeStatus::Failed => 4,
            NodeStatus::Skipped => 5,
            NodeStatus::Cancelled => 6,
        };
        self.status.store(value, Ordering::SeqCst);
    }

    /// Get the current in-degree value
    pub fn get_in_degree(&self) -> usize {
        self.in_degree.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_in_degree_management() {
        let mut node = Node::new(NodeId::new(), "Test Node".to_string());
        assert_eq!(node.in_degree, 0);
        assert_eq!(node.status, NodeStatus::Pending);

        // Add some dependencies
        node.increment_in_degree();
        node.increment_in_degree();
        assert_eq!(node.in_degree, 2);

        // Remove one dependency
        let became_ready = node.decrement_in_degree();
        assert_eq!(became_ready, false);
        assert_eq!(node.in_degree, 1);
        assert_eq!(node.status, NodeStatus::Pending);

        // Remove last dependency, should become ready
        let became_ready = node.decrement_in_degree();
        assert_eq!(became_ready, true);
        assert_eq!(node.in_degree, 0);
        assert_eq!(node.status, NodeStatus::Ready);
    }

    #[test]
    fn test_atomic_node() {
        let mut node = Node::new(NodeId::new(), "Test Node".to_string());
        node.increment_in_degree();
        node.increment_in_degree();

        let atomic_node = AtomicNode::new(&node);
        assert_eq!(atomic_node.get_in_degree(), 2);
        assert_eq!(atomic_node.get_status(), NodeStatus::Pending);

        // Test atomic operations
        let became_ready = atomic_node.decrement_in_degree();
        assert_eq!(became_ready, false);
        assert_eq!(atomic_node.get_in_degree(), 1);

        let became_ready = atomic_node.decrement_in_degree();
        assert_eq!(became_ready, true);
        assert_eq!(atomic_node.get_in_degree(), 0);
        assert_eq!(atomic_node.get_status(), NodeStatus::Ready);
    }
}
