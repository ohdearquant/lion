use crate::model::edge::{Edge, EdgeId};
use crate::model::node::{Node, NodeId, NodeStatus};
use lion_capability::model::capability::CapabilityId;
use lion_core::id::Id;
use lion_core::error::Error as CoreError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;
use uuid::Uuid;

/// Unique identifier for workflows
pub type WorkflowId = Id<WorkflowDefinition>;

/// Error types for workflow operations
#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),
    
    #[error("Edge not found: {0}")]
    EdgeNotFound(EdgeId),
    
    #[error("Cycle detected in workflow graph")]
    CycleDetected,
    
    #[error("Duplicate node ID: {0}")]
    DuplicateNode(NodeId),
    
    #[error("Duplicate edge ID: {0}")]
    DuplicateEdge(EdgeId),
    
    #[error("Invalid edge: source node {0} not found")]
    InvalidEdgeSource(NodeId),
    
    #[error("Invalid edge: target node {0} not found")]
    InvalidEdgeTarget(NodeId),
    
    #[error("Capability violation: {0}")]
    CapabilityViolation(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Workflow validation error: {0}")]
    ValidationError(String),
    
    #[error("Core error: {0}")]
    CoreError(#[from] CoreError),
}

/// Version information for workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Major version (incremented for breaking changes)
    pub major: u32,
    /// Minor version (incremented for compatible changes)
    pub minor: u32,
    /// Patch version (incremented for bug fixes)
    pub patch: u32,
}

impl Default for Version {
    fn default() -> Self {
        Version {
            major: 1,
            minor: 0,
            patch: 0,
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Definition of a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique identifier for this workflow
    pub id: WorkflowId,
    
    /// User-friendly name of this workflow
    pub name: String,
    
    /// Optional description
    pub description: Option<String>,
    
    /// Version of this workflow
    pub version: Version,
    
    /// Nodes in this workflow (adjacency list representation)
    pub nodes: HashMap<NodeId, Node>,
    
    /// Edges in this workflow
    pub edges: HashMap<EdgeId, Edge>,
    
    /// Map of node IDs with no incoming edges (start nodes)
    pub start_nodes: HashSet<NodeId>,
    
    /// Map of node IDs with no outgoing edges (end nodes)
    pub end_nodes: HashSet<NodeId>,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    
    /// Capability required to execute this workflow
    pub required_capability: Option<CapabilityId>,
}

impl WorkflowDefinition {
    /// Create a new empty workflow definition
    pub fn new(id: WorkflowId, name: String) -> Self {
        let now = chrono::Utc::now();
        
        WorkflowDefinition {
            id,
            name,
            description: None,
            version: Version::default(),
            nodes: HashMap::new(),
            edges: HashMap::new(),
            start_nodes: HashSet::new(),
            end_nodes: HashSet::new(),
            created_at: now,
            updated_at: now,
            required_capability: None,
        }
    }
    
    /// Add a node to this workflow
    pub fn add_node(&mut self, node: Node) -> Result<(), WorkflowError> {
        if self.nodes.contains_key(&node.id) {
            return Err(WorkflowError::DuplicateNode(node.id));
        }
        
        // Initially, every node is both a start and end node
        // We'll update these sets when edges are added
        self.start_nodes.insert(node.id);
        self.end_nodes.insert(node.id);
        
        self.nodes.insert(node.id, node);
        self.updated_at = chrono::Utc::now();
        
        Ok(())
    }
    
    /// Add an edge to this workflow
    pub fn add_edge(&mut self, edge: Edge) -> Result<(), WorkflowError> {
        if self.edges.contains_key(&edge.id) {
            return Err(WorkflowError::DuplicateEdge(edge.id));
        }
        
        // Verify that source and target nodes exist
        if !self.nodes.contains_key(&edge.source) {
            return Err(WorkflowError::InvalidEdgeSource(edge.source));
        }
        
        if !self.nodes.contains_key(&edge.target) {
            return Err(WorkflowError::InvalidEdgeTarget(edge.target));
        }
        
        // Check for capability violations (cross-boundary edges)
        self.validate_edge_capabilities(&edge)?;
        
        // Update the source node (add outgoing edge)
        if let Some(source_node) = self.nodes.get_mut(&edge.source) {
            source_node.add_outgoing_edge(edge.id);
            // Source node is no longer an end node
            self.end_nodes.remove(&edge.source);
        }
        
        // Update the target node (add incoming edge)
        if let Some(target_node) = self.nodes.get_mut(&edge.target) {
            target_node.add_incoming_edge(edge.id);
            // Target node is no longer a start node
            self.start_nodes.remove(&edge.target);
        }
        
        // Add the edge to our map
        self.edges.insert(edge.id, edge);
        self.updated_at = chrono::Utc::now();
        
        // Check if the new edge creates a cycle
        if self.has_cycle() {
            // If it does, revert our changes
            self.remove_edge(&edge.id)?;
            return Err(WorkflowError::CycleDetected);
        }
        
        Ok(())
    }
    
    /// Remove a node from this workflow
    pub fn remove_node(&mut self, node_id: &NodeId) -> Result<Node, WorkflowError> {
        let node = self.nodes.remove(node_id)
            .ok_or_else(|| WorkflowError::NodeNotFound(*node_id))?;
        
        // Remove all edges connected to this node
        let mut edges_to_remove = Vec::new();
        
        // Find all edges that connect to this node
        for edge in self.edges.values() {
            if edge.source == *node_id || edge.target == *node_id {
                edges_to_remove.push(edge.id);
            }
        }
        
        // Remove the edges
        for edge_id in edges_to_remove {
            self.remove_edge(&edge_id)?;
        }
        
        // Update start/end node sets
        self.start_nodes.remove(node_id);
        self.end_nodes.remove(node_id);
        
        self.updated_at = chrono::Utc::now();
        
        Ok(node)
    }
    
    /// Remove an edge from this workflow
    pub fn remove_edge(&mut self, edge_id: &EdgeId) -> Result<Edge, WorkflowError> {
        let edge = self.edges.remove(edge_id)
            .ok_or_else(|| WorkflowError::EdgeNotFound(*edge_id))?;
        
        // Update the source node
        if let Some(source_node) = self.nodes.get_mut(&edge.source) {
            source_node.remove_outgoing_edge(edge_id);
            // Check if the source node is now an end node
            if source_node.outgoing_edges.is_empty() {
                self.end_nodes.insert(edge.source);
            }
        }
        
        // Update the target node
        if let Some(target_node) = self.nodes.get_mut(&edge.target) {
            target_node.remove_incoming_edge(edge_id);
            // Check if the target node is now a start node
            if target_node.incoming_edges.is_empty() {
                self.start_nodes.insert(edge.target);
            }
        }
        
        self.updated_at = chrono::Utc::now();
        
        Ok(edge)
    }
    
    /// Get a node by its ID
    pub fn get_node(&self, node_id: &NodeId) -> Option<&Node> {
        self.nodes.get(node_id)
    }
    
    /// Get a mutable reference to a node by its ID
    pub fn get_node_mut(&mut self, node_id: &NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(node_id)
    }
    
    /// Get an edge by its ID
    pub fn get_edge(&self, edge_id: &EdgeId) -> Option<&Edge> {
        self.edges.get(edge_id)
    }
    
    /// Get all child nodes of a given node
    pub fn get_child_nodes(&self, node_id: &NodeId) -> Result<Vec<&Node>, WorkflowError> {
        let node = self.nodes.get(node_id)
            .ok_or_else(|| WorkflowError::NodeNotFound(*node_id))?;
        
        let mut children = Vec::new();
        
        // For each outgoing edge, find the target node
        for edge_id in &node.outgoing_edges {
            if let Some(edge) = self.edges.get(edge_id) {
                if let Some(child_node) = self.nodes.get(&edge.target) {
                    children.push(child_node);
                }
            }
        }
        
        Ok(children)
    }
    
    /// Get all parent nodes of a given node
    pub fn get_parent_nodes(&self, node_id: &NodeId) -> Result<Vec<&Node>, WorkflowError> {
        let node = self.nodes.get(node_id)
            .ok_or_else(|| WorkflowError::NodeNotFound(*node_id))?;
        
        let mut parents = Vec::new();
        
        // For each incoming edge, find the source node
        for edge_id in &node.incoming_edges {
            if let Some(edge) = self.edges.get(edge_id) {
                if let Some(parent_node) = self.nodes.get(&edge.source) {
                    parents.push(parent_node);
                }
            }
        }
        
        Ok(parents)
    }
    
    /// Get the outgoing edges from a node
    pub fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<&Edge>, WorkflowError> {
        let node = self.nodes.get(node_id)
            .ok_or_else(|| WorkflowError::NodeNotFound(*node_id))?;
        
        let mut edges = Vec::new();
        
        for edge_id in &node.outgoing_edges {
            if let Some(edge) = self.edges.get(edge_id) {
                edges.push(edge);
            }
        }
        
        Ok(edges)
    }
    
    /// Get the incoming edges to a node
    pub fn get_incoming_edges(&self, node_id: &NodeId) -> Result<Vec<&Edge>, WorkflowError> {
        let node = self.nodes.get(node_id)
            .ok_or_else(|| WorkflowError::NodeNotFound(*node_id))?;
        
        let mut edges = Vec::new();
        
        for edge_id in &node.incoming_edges {
            if let Some(edge) = self.edges.get(edge_id) {
                edges.push(edge);
            }
        }
        
        Ok(edges)
    }
    
    /// Check if the workflow graph has a cycle
    /// Uses depth-first search to detect cycles
    pub fn has_cycle(&self) -> bool {
        if self.nodes.is_empty() {
            return false;
        }
        
        // Track visited and currently in stack nodes
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        
        // Start DFS from each unvisited node
        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                if self.has_cycle_dfs(node_id, &mut visited, &mut in_stack) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Helper for cycle detection using DFS
    fn has_cycle_dfs(
        &self,
        node_id: &NodeId,
        visited: &mut HashSet<NodeId>,
        in_stack: &mut HashSet<NodeId>,
    ) -> bool {
        // Mark current node as visited and add to recursion stack
        visited.insert(*node_id);
        in_stack.insert(*node_id);
        
        // Visit all neighbors
        if let Some(node) = self.nodes.get(node_id) {
            for edge_id in &node.outgoing_edges {
                if let Some(edge) = self.edges.get(edge_id) {
                    // If child is not visited, recursively check
                    if !visited.contains(&edge.target) {
                        if self.has_cycle_dfs(&edge.target, visited, in_stack) {
                            return true;
                        }
                    } else if in_stack.contains(&edge.target) {
                        // If child is in recursion stack, there's a cycle
                        return true;
                    }
                }
            }
        }
        
        // Remove node from recursion stack
        in_stack.remove(node_id);
        false
    }
    
    /// Find all nodes without dependencies (in-degree = 0)
    pub fn get_ready_nodes(&self) -> Vec<&Node> {
        self.nodes.values()
            .filter(|node| node.in_degree == 0 && node.status == NodeStatus::Pending)
            .collect()
    }
    
    /// Get a topological sort order of the workflow nodes
    /// Returns nodes in an order where each node appears after all its dependencies
    pub fn get_topological_order(&self) -> Result<Vec<NodeId>, WorkflowError> {
        if self.has_cycle() {
            return Err(WorkflowError::CycleDetected);
        }
        
        let mut result = Vec::new();
        let mut in_degree = HashMap::new();
        let mut queue = VecDeque::new();
        
        // Initialize in-degree for all nodes
        for (id, node) in &self.nodes {
            in_degree.insert(*id, node.in_degree);
            if node.in_degree == 0 {
                queue.push_back(*id);
            }
        }
        
        // Process nodes with in-degree 0
        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);
            
            // Reduce in-degree of all neighbor nodes
            if let Some(node) = self.nodes.get(&node_id) {
                for edge_id in &node.outgoing_edges {
                    if let Some(edge) = self.edges.get(edge_id) {
                        if let Some(degree) = in_degree.get_mut(&edge.target) {
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push_back(edge.target);
                            }
                        }
                    }
                }
            }
        }
        
        // If we visited all nodes, return the result
        if result.len() == self.nodes.len() {
            Ok(result)
        } else {
            Err(WorkflowError::CycleDetected)
        }
    }
    
    /// Validate that all edges respect capability boundaries
    fn validate_edge_capabilities(&self, edge: &Edge) -> Result<(), WorkflowError> {
        // If no capabilities are involved, no validation needed
        if edge.required_capability.is_none() {
            return Ok(());
        }
        
        // Check if source and target have compatible capabilities
        let source_cap = self.nodes.get(&edge.source)
            .and_then(|node| node.required_capability);
        
        let target_cap = self.nodes.get(&edge.target)
            .and_then(|node| node.required_capability);
        
        // If the nodes have different capabilities, the edge must have a capability
        if source_cap != target_cap && edge.required_capability.is_none() {
            return Err(WorkflowError::CapabilityViolation(
                format!("Edge from {:?} to {:?} crosses capability boundaries without an explicit capability",
                        edge.source, edge.target)
            ));
        }
        
        Ok(())
    }
    
    /// Set the required capability for this workflow
    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.required_capability = Some(capability_id);
        self
    }
    
    /// Set the description for this workflow
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
    
    /// Set the version for this workflow
    pub fn with_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.version = Version { major, minor, patch };
        self
    }
    
    /// Reset the state of all nodes to Pending
    pub fn reset(&mut self) {
        for node in self.nodes.values_mut() {
            node.status = NodeStatus::Pending;
        }
    }
    
    /// Serialize this workflow to JSON
    pub fn to_json(&self) -> Result<String, WorkflowError> {
        serde_json::to_string(self)
            .map_err(|e| WorkflowError::SerializationError(e.to_string()))
    }
    
    /// Deserialize a workflow from JSON
    pub fn from_json(json: &str) -> Result<Self, WorkflowError> {
        serde_json::from_str(json)
            .map_err(|e| WorkflowError::SerializationError(e.to_string()))
    }
}

/// Builder for workflow definitions
pub struct WorkflowBuilder {
    definition: WorkflowDefinition,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(name: &str) -> Self {
        let id = WorkflowId::new();
        WorkflowBuilder {
            definition: WorkflowDefinition::new(id, name.to_string()),
        }
    }
    
    /// Set the description for this workflow
    pub fn description(mut self, description: &str) -> Self {
        self.definition.description = Some(description.to_string());
        self
    }
    
    /// Set the version for this workflow
    pub fn version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.definition.version = Version { major, minor, patch };
        self
    }
    
    /// Set the required capability for this workflow
    pub fn capability(mut self, capability_id: CapabilityId) -> Self {
        self.definition.required_capability = Some(capability_id);
        self
    }
    
    /// Add a node to this workflow
    pub fn add_node(mut self, node: Node) -> Result<Self, WorkflowError> {
        self.definition.add_node(node)?;
        Ok(self)
    }
    
    /// Add an edge to this workflow
    pub fn add_edge(mut self, edge: Edge) -> Result<Self, WorkflowError> {
        self.definition.add_edge(edge)?;
        Ok(self)
    }
    
    /// Build the workflow definition
    pub fn build(self) -> WorkflowDefinition {
        self.definition
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_workflow_creation() {
        let workflow = WorkflowDefinition::new(WorkflowId::new(), "Test Workflow".to_string());
        
        assert_eq!(workflow.name, "Test Workflow");
        assert!(workflow.nodes.is_empty());
        assert!(workflow.edges.is_empty());
        assert!(workflow.start_nodes.is_empty());
        assert!(workflow.end_nodes.is_empty());
    }
    
    #[test]
    fn test_add_nodes_and_edges() {
        let mut workflow = WorkflowDefinition::new(WorkflowId::new(), "Test Workflow".to_string());
        
        // Add two nodes
        let node1 = Node::new(NodeId::new(), "Node 1".to_string());
        let node2 = Node::new(NodeId::new(), "Node 2".to_string());
        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();
        
        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        
        assert_eq!(workflow.nodes.len(), 2);
        assert_eq!(workflow.start_nodes.len(), 2);
        assert_eq!(workflow.end_nodes.len(), 2);
        
        // Add an edge
        let edge = Edge::new(EdgeId::new(), &node1_id, &node2_id);
        let edge_id = edge.id;
        
        workflow.add_edge(edge).unwrap();
        
        assert_eq!(workflow.edges.len(), 1);
        assert_eq!(workflow.start_nodes.len(), 1); // Only node1 is a start node
        assert_eq!(workflow.end_nodes.len(), 1);   // Only node2 is an end node
        
        // Check node connections
        let node1 = workflow.get_node(&node1_id).unwrap();
        let node2 = workflow.get_node(&node2_id).unwrap();
        
        assert_eq!(node1.outgoing_edges.len(), 1);
        assert_eq!(node2.incoming_edges.len(), 1);
        assert!(node1.outgoing_edges.contains(&edge_id));
        assert!(node2.incoming_edges.contains(&edge_id));
    }
    
    #[test]
    fn test_cycle_detection() {
        let mut workflow = WorkflowDefinition::new(WorkflowId::new(), "Test Workflow".to_string());
        
        // Add three nodes
        let node1 = Node::new(NodeId::new(), "Node 1".to_string());
        let node2 = Node::new(NodeId::new(), "Node 2".to_string());
        let node3 = Node::new(NodeId::new(), "Node 3".to_string());
        
        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();
        let node3_id = node3.id.clone();
        
        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        workflow.add_node(node3).unwrap();
        
        // Add edges to form a cycle: 1 -> 2 -> 3 -> 1
        workflow.add_edge(Edge::new(EdgeId::new(), &node1_id, &node2_id)).unwrap();
        workflow.add_edge(Edge::new(EdgeId::new(), &node2_id, &node3_id)).unwrap();
        
        // This edge creates a cycle and should fail
        let result = workflow.add_edge(Edge::new(EdgeId::new(), &node3_id, &node1_id));
        
        assert!(result.is_err());
        match result {
            Err(WorkflowError::CycleDetected) => (),
            _ => panic!("Expected CycleDetected error"),
        }
    }
    
    #[test]
    fn test_topological_sort() {
        let mut workflow = WorkflowDefinition::new(WorkflowId::new(), "Test Workflow".to_string());
        
        // Add four nodes
        let node1 = Node::new(NodeId::new(), "Node 1".to_string());
        let node2 = Node::new(NodeId::new(), "Node 2".to_string());
        let node3 = Node::new(NodeId::new(), "Node 3".to_string());
        let node4 = Node::new(NodeId::new(), "Node 4".to_string());
        
        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();
        let node3_id = node3.id.clone();
        let node4_id = node4.id.clone();
        
        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        workflow.add_node(node3).unwrap();
        workflow.add_node(node4).unwrap();
        
        // Add edges to form a DAG
        // 1 -> 2 -> 4
        //  \-> 3 -/
        workflow.add_edge(Edge::new(EdgeId::new(), &node1_id, &node2_id)).unwrap();
        workflow.add_edge(Edge::new(EdgeId::new(), &node1_id, &node3_id)).unwrap();
        workflow.add_edge(Edge::new(EdgeId::new(), &node2_id, &node4_id)).unwrap();
        workflow.add_edge(Edge::new(EdgeId::new(), &node3_id, &node4_id)).unwrap();
        
        let order = workflow.get_topological_order().unwrap();
        
        // Valid topological orders:
        // [1, 2, 3, 4] or [1, 3, 2, 4]
        
        // Check that the order is valid
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], node1_id); // Node 1 must be first
        assert_eq!(order[3], node4_id); // Node 4 must be last
        
        // Node 2 and 3 can be in either order, but before 4
        let idx2 = order.iter().position(|id| *id == node2_id).unwrap();
        let idx3 = order.iter().position(|id| *id == node3_id).unwrap();
        assert!(idx2 > 0 && idx2 < 3);
        assert!(idx3 > 0 && idx3 < 3);
    }
    
    #[test]
    fn test_capability_validation() {
        let mut workflow = WorkflowDefinition::new(WorkflowId::new(), "Test Workflow".to_string());
        
        let cap1 = CapabilityId::new();
        let cap2 = CapabilityId::new();
        
        // Create nodes with different capabilities
        let mut node1 = Node::new(NodeId::new(), "Node 1".to_string());
        let mut node2 = Node::new(NodeId::new(), "Node 2".to_string());
        
        node1.required_capability = Some(cap1);
        node2.required_capability = Some(cap2);
        
        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();
        
        workflow.add_node(node1).unwrap();
        workflow.add_node(node2).unwrap();
        
        // Create an edge without a capability (should fail)
        let edge_without_cap = Edge::new(EdgeId::new(), &node1_id, &node2_id);
        let result = workflow.add_edge(edge_without_cap);
        
        assert!(result.is_err());
        match result {
            Err(WorkflowError::CapabilityViolation(_)) => (),
            _ => panic!("Expected CapabilityViolation error"),
        }
        
        // Create an edge with a capability (should succeed)
        let edge_with_cap = Edge::new(EdgeId::new(), &node1_id, &node2_id)
            .with_capability(CapabilityId::new());
        
        let result = workflow.add_edge(edge_with_cap);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_builder_pattern() {
        let cap = CapabilityId::new();
        let node1_id = NodeId::new();
        let node2_id = NodeId::new();
        
        let workflow = WorkflowBuilder::new("Test Workflow")
            .description("A test workflow")
            .version(1, 2, 3)
            .capability(cap)
            .add_node(Node::new(node1_id, "Node 1".to_string())).unwrap()
            .add_node(Node::new(node2_id, "Node 2".to_string())).unwrap()
            .add_edge(Edge::new(EdgeId::new(), &node1_id, &node2_id)).unwrap()
            .build();
        
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.description, Some("A test workflow".to_string()));
        assert_eq!(workflow.version.major, 1);
        assert_eq!(workflow.version.minor, 2);
        assert_eq!(workflow.version.patch, 3);
        assert_eq!(workflow.required_capability, Some(cap));
        assert_eq!(workflow.nodes.len(), 2);
        assert_eq!(workflow.edges.len(), 1);
    }
}