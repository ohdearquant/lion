use crate::workflows::{WorkflowDefinition, WorkflowEdge, WorkflowNode};
use std::collections::HashMap;

// Simplified version that doesn't depend on external crates
pub struct SimpleWorkflowGraph {
    nodes: HashMap<String, SimpleNode>,
    edges: Vec<SimpleEdge>,
}

pub struct SimpleNode {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub data: serde_json::Value,
}

pub struct SimpleEdge {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
}

/// Converts the UI's WorkflowDefinition to a simplified workflow graph.
///
/// # Arguments
/// * `definition` - The UI workflow definition to convert
///
/// # Returns
/// * `Result<SimpleWorkflowGraph, String>` - The converted graph or an error
pub fn convert_to_simple_graph(
    definition: &WorkflowDefinition,
) -> Result<SimpleWorkflowGraph, String> {
    let mut nodes = HashMap::new();
    let mut edges = Vec::new();

    // Convert Nodes
    for ui_node in &definition.nodes {
        let node_type = ui_node.node_type.as_deref().unwrap_or("task").to_string();

        let name = ui_node
            .data
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed Node")
            .to_string();

        let node = SimpleNode {
            id: ui_node.id.clone(),
            node_type,
            name,
            data: ui_node.data.clone(),
        };

        nodes.insert(ui_node.id.clone(), node);
    }

    // Convert Edges
    for ui_edge in &definition.edges {
        let edge = SimpleEdge {
            source: ui_edge.source.clone(),
            target: ui_edge.target.clone(),
            label: ui_edge.label.clone(),
        };

        edges.push(edge);
    }

    // Create the workflow graph
    let graph = SimpleWorkflowGraph { nodes, edges };

    Ok(graph)
}

/// Converts a workflow definition to a runtime-compatible graph format.
/// This is used when sending workflow definitions to the runtime engine.
///
/// # Arguments
/// * `definition` - The workflow definition to convert
///
/// # Returns
/// * `Result<serde_json::Value, String>` - The runtime graph or an error
pub fn convert_to_runtime_graph(
    definition: &WorkflowDefinition,
) -> Result<serde_json::Value, String> {
    // First convert to a simple graph
    let simple_graph = convert_to_simple_graph(definition)?;

    // Create runtime nodes array
    let mut runtime_nodes = Vec::new();
    for (_, node) in simple_graph.nodes.iter() {
        let runtime_node = serde_json::json!({
            "id": node.id,
            "type": node.node_type,
            "name": node.name,
            "properties": node.data,
        });
        runtime_nodes.push(runtime_node);
    }

    // Create runtime connections array
    let mut runtime_connections = Vec::new();
    for edge in simple_graph.edges.iter() {
        let runtime_edge = serde_json::json!({
            "source": edge.source,
            "target": edge.target,
            "condition": edge.label.clone().unwrap_or_else(|| "default".to_string()),
        });
        runtime_connections.push(runtime_edge);
    }

    // Create the final runtime graph
    let runtime_graph = serde_json::json!({
        "id": definition.id,
        "name": definition.name,
        "description": definition.description,
        "version": definition.version,
        "nodes": runtime_nodes,
        "connections": runtime_connections,
        "metadata": {
            "created_at": definition.created_at,
            "updated_at": definition.updated_at,
        }
    });

    Ok(runtime_graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows::Position;
    use serde_json::json;

    #[test]
    fn test_convert_simple_workflow() {
        let definition = WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "Test workflow description".to_string(),
            version: "1.0.0".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "node-1".to_string(),
                    node_type: Some("task".to_string()),
                    data: json!({
                        "label": "Task 1",
                        "taskType": "http_request"
                    }),
                    position: Position { x: 0.0, y: 0.0 },
                },
                WorkflowNode {
                    id: "node-2".to_string(),
                    node_type: Some("task".to_string()),
                    data: json!({
                        "label": "Task 2",
                        "taskType": "transform"
                    }),
                    position: Position { x: 100.0, y: 0.0 },
                },
            ],
            edges: vec![WorkflowEdge {
                id: "edge-1".to_string(),
                source: "node-1".to_string(),
                target: "node-2".to_string(),
                edge_type: None,
                label: Some("success".to_string()),
                animated: None,
            }],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            file_path: None,
        };

        let result = convert_to_simple_graph(&definition);
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_convert_to_runtime_graph() {
        let definition = WorkflowDefinition {
            id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "Test workflow description".to_string(),
            version: "1.0.0".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "node-1".to_string(),
                    node_type: Some("task".to_string()),
                    data: json!({
                        "label": "Task 1",
                        "taskType": "http_request"
                    }),
                    position: Position { x: 0.0, y: 0.0 },
                },
                WorkflowNode {
                    id: "node-2".to_string(),
                    node_type: Some("task".to_string()),
                    data: json!({
                        "label": "Task 2",
                        "taskType": "transform"
                    }),
                    position: Position { x: 100.0, y: 0.0 },
                },
            ],
            edges: vec![WorkflowEdge {
                id: "edge-1".to_string(),
                source: "node-1".to_string(),
                target: "node-2".to_string(),
                edge_type: None,
                label: Some("success".to_string()),
                animated: None,
            }],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            file_path: None,
        };

        let result = convert_to_runtime_graph(&definition);
        assert!(result.is_ok());

        let runtime_graph = result.unwrap();
        assert!(runtime_graph.is_object());

        // Check that the runtime graph has the expected structure
        assert_eq!(runtime_graph["id"], "test-workflow");
        assert_eq!(runtime_graph["name"], "Test Workflow");
        // Check nodes
        let nodes = runtime_graph["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);

        // Find node-1 and node-2 in the nodes array (order may not be guaranteed)
        let node1 = nodes
            .iter()
            .find(|n| n["id"] == "node-1")
            .expect("Node-1 should exist");
        let node2 = nodes
            .iter()
            .find(|n| n["id"] == "node-2")
            .expect("Node-2 should exist");

        // Verify node properties
        assert_eq!(node1["type"], "task");
        assert_eq!(node2["type"], "task");
        assert_eq!(nodes[0]["type"], "task");

        // Check connections
        let connections = runtime_graph["connections"].as_array().unwrap();
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0]["source"], "node-1");
        assert_eq!(connections[0]["target"], "node-2");
        assert_eq!(connections[0]["condition"], "success");
    }
}
