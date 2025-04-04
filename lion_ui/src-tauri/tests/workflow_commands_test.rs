use lion_ui_tauri::state::WorkflowManager;
use lion_ui_tauri::workflows::{Position, WorkflowDefinition, WorkflowEdge, WorkflowNode};
use std::sync::Arc;

#[tokio::test]
async fn test_workflow_manager() {
    // Create a workflow manager
    let workflow_manager = WorkflowManager::new();

    // Create a test workflow definition
    let workflow_def = WorkflowDefinition {
        id: "test-workflow".to_string(),
        name: "Test Workflow".to_string(),
        description: "A test workflow".to_string(),
        version: "1.0.0".to_string(),
        nodes: vec![WorkflowNode {
            id: "start".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            node_type: Some("start".to_string()),
            data: serde_json::json!({}),
        }],
        edges: vec![WorkflowEdge {
            id: "edge1".to_string(),
            source: "start".to_string(),
            target: "end".to_string(),
            edge_type: None,
            label: None,
            animated: None,
        }],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        file_path: None,
    };

    // Add the workflow definition to the manager
    {
        let mut definitions = workflow_manager.definitions.lock().await;
        definitions.push(workflow_def.clone());
    }

    // Verify the workflow was added
    let definitions = workflow_manager.definitions.lock().await;
    assert_eq!(definitions.len(), 1, "Workflow should be added");
    assert_eq!(
        definitions[0].id, "test-workflow",
        "Workflow ID should match"
    );
    assert_eq!(
        definitions[0].name, "Test Workflow",
        "Workflow name should match"
    );
    drop(definitions);

    // Add an instance
    {
        let mut instances = workflow_manager.instances.lock().await;
        instances.push("instance-1".to_string());
    }

    // Verify the instance was added
    let instances = workflow_manager.instances.lock().await;
    assert_eq!(instances.len(), 1, "Instance should be added");
    assert_eq!(instances[0], "instance-1", "Instance ID should match");
}
