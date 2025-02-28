use std::collections::{HashMap, HashSet};
use chrono::Utc;
use lion_core::id::Id;
use lion_workflow::model::node::NodeId;
use lion_workflow::model::definition::{WorkflowDefinition, Version};
use lion_workflow::model::edge::{Edge, EdgeId};
use lion_workflow::model::node::{Node, NodeStatus, Priority};

/// Test function to check if a workflow is acyclic
fn is_acyclic(definition: &WorkflowDefinition) -> bool {
    // Count in-degrees for each node
    let mut in_degree = HashMap::new();
    
    for (node_id, _) in &definition.nodes {
        in_degree.insert(node_id.clone(), 0);
    }

    // Count incoming edges for each node
    for (_, edge) in &definition.edges {
        if let Some(count) = in_degree.get_mut(&edge.target) {
            *count += 1;
        }
    }

    // Kahn's algorithm
    let mut q = std::collections::VecDeque::new();

    // Add nodes with no dependencies
    for (id, &count) in &in_degree {
        if count == 0 {
            q.push_back(id.clone());
        }
    }

    let mut visited_count = 0;

    while let Some(node_id) = q.pop_front() {
        visited_count += 1;

        // Find edges that start from this node
        for (_, edge) in &definition.edges {
            if edge.source == node_id {
                // This edge goes from the current node to another
                if let Some(count) = in_degree.get_mut(&edge.target) {
                    *count -= 1;
                    if *count == 0 {
                        q.push_back(edge.target.clone());
                    }
                }
            }
        }
    }

    // If we visited all nodes, there are no cycles
    visited_count == definition.nodes.len()
}

fn create_acyclic_workflow() -> WorkflowDefinition {
    let mut nodes = HashMap::new();
    let mut edges = HashMap::new();
    let mut start_nodes = HashSet::new();
    let mut end_nodes = HashSet::new();

    // Create nodes
    let node1_id = NodeId::new();
    let node1 = Node {
        id: node1_id.clone(),
        name: "Node 1".to_string(),
        status: NodeStatus::Pending,
        in_degree: 0,
        outgoing_edges: HashSet::new(),
        incoming_edges: HashSet::new(),
        required_capability: None,
        priority: Priority::Normal,
        deadline: None, 
        config: serde_json::Value::Null,
    };

    let node2_id = NodeId::new();
    let node2 = Node {
        id: node2_id.clone(),
        name: "Node 2".to_string(),
        status: NodeStatus::Pending,
        in_degree: 0,
        outgoing_edges: HashSet::new(),
        incoming_edges: HashSet::new(),
        required_capability: None,
        priority: Priority::Normal,
        deadline: None,
        config: serde_json::Value::Null,
    };

    let node3_id = NodeId::new();
    let node3 = Node {
        id: node3_id.clone(),
        name: "Node 3".to_string(),
        status: NodeStatus::Pending,
        in_degree: 0,
        outgoing_edges: HashSet::new(),
        incoming_edges: HashSet::new(),
        required_capability: None,
        priority: Priority::Normal,
        deadline: None,
        config: serde_json::Value::Null,
    };

    // Create edges for a DAG: 1 -> 2 -> 3
    // Use Edge::new() constructor which properly initializes all fields
    let edge1 = Edge::new(EdgeId::new(), node1_id.clone(), node2_id.clone());
    let edge1_id = edge1.id.clone();

    // Use Edge::new() constructor which properly initializes all fields
    let edge2 = Edge::new(EdgeId::new(), node2_id.clone(), node3_id.clone());
    let edge2_id = edge2.id.clone();

    // Add edges to the map
    edges.insert(edge1_id, edge1);
    edges.insert(edge2_id, edge2);

    // Add nodes to the map
    nodes.insert(node1_id.clone(), node1);
    nodes.insert(node2_id.clone(), node2);
    nodes.insert(node3_id.clone(), node3);

    // Set start and end nodes
    start_nodes.insert(node1_id);
    end_nodes.insert(node3_id);
    
    WorkflowDefinition {
        id: Id::new(),
        name: "Acyclic".to_string(),
        description: Some("Acyclic workflow".to_string()),
        version: Version::default(),
        nodes,
        edges,
        start_nodes,
        end_nodes,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        required_capability: None,
    }
}

fn create_cyclic_workflow() -> WorkflowDefinition {
    let mut nodes = HashMap::new();
    let mut edges = HashMap::new();
    let start_nodes = HashSet::new();
    let end_nodes = HashSet::new();

    // Create nodes
    let node1_id = NodeId::new();
    let node1 = Node {
        id: node1_id.clone(),
        name: "Node 1".to_string(),
        status: NodeStatus::Pending,
        in_degree: 0,
        outgoing_edges: HashSet::new(),
        incoming_edges: HashSet::new(),
        required_capability: None,
        priority: Priority::Normal,
        deadline: None,
        config: serde_json::Value::Null,
    };

    let node2_id = NodeId::new();
    let node2 = Node {
        id: node2_id.clone(),
        name: "Node 2".to_string(),
        status: NodeStatus::Pending,
        in_degree: 0,
        outgoing_edges: HashSet::new(),
        incoming_edges: HashSet::new(),
        required_capability: None,
        priority: Priority::Normal,
        deadline: None,
        config: serde_json::Value::Null,
    };

    let node3_id = NodeId::new();
    let node3 = Node {
        id: node3_id.clone(),
        name: "Node 3".to_string(),
        status: NodeStatus::Pending,
        in_degree: 0,
        outgoing_edges: HashSet::new(),
        incoming_edges: HashSet::new(),
        required_capability: None,
        priority: Priority::Normal,
        deadline: None,
        config: serde_json::Value::Null,
    };

    // Create edges for a cycle: 1 -> 2 -> 3 -> 1
    // Use Edge::new() constructor which properly initializes all fields
    let edge1 = Edge::new(EdgeId::new(), node1_id.clone(), node2_id.clone());
    let edge1_id = edge1.id.clone();

    // Use Edge::new() constructor which properly initializes all fields
    let edge2 = Edge::new(EdgeId::new(), node2_id.clone(), node3_id.clone());
    let edge2_id = edge2.id.clone();

    // Use Edge::new() constructor which properly initializes all fields - this creates the cycle
    let edge3 = Edge::new(EdgeId::new(), node3_id.clone(), node1_id.clone());
    let edge3_id = edge3.id.clone();

    // Add edges to the map
    edges.insert(edge1_id, edge1);
    edges.insert(edge2_id, edge2);
    edges.insert(edge3_id, edge3);

    // Add nodes to the map
    nodes.insert(node1_id, node1);
    nodes.insert(node2_id, node2);
    nodes.insert(node3_id, node3);

    WorkflowDefinition {
        id: Id::new(),
        name: "Cyclic".to_string(),
        description: Some("Cyclic workflow".to_string()),
        version: Version::default(),
        nodes,
        edges,
        start_nodes,
        end_nodes,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        required_capability: None,
    }
}

#[test]
fn test_acyclic_check() {
    // Create workflows
    let acyclic = create_acyclic_workflow();
    let cyclic = create_cyclic_workflow();

    // Check acyclic
    assert!(is_acyclic(&acyclic));

    // Check cyclic
    assert!(!is_acyclic(&cyclic));
}