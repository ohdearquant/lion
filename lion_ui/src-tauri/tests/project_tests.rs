use lion_ui_tauri::project::{Project, ProjectState};
use lion_ui_tauri::runtime::RuntimeState;
use std::path::{Path, PathBuf};

#[tokio::test]
async fn test_project_creation() {
    // Create a default project to test fields
    let project = Project::default();

    // Check default values
    assert_eq!(project.name, "");
    assert_eq!(project.root_path, "");
    assert!(project.folders.is_empty());
    assert!(!project.is_loaded);
}

#[tokio::test]
async fn test_project_state() {
    // Create a new project state
    let project_state = ProjectState::new();

    // Initially, current project should be None
    let current_project = project_state.current_project.lock().await;
    assert!(current_project.is_none());
}

// Mock function for gather_project_folders
fn mock_gather_folders(_path: &Path) -> Result<Vec<String>, String> {
    Ok(vec![
        "src".to_string(),
        "tests".to_string(),
        "docs".to_string(),
    ])
}

// Custom test to mock project loading functionality
// Normally this would use the real filesystem, but we mock it for testing
#[tokio::test]
async fn test_project_loading_mock() {
    // Create project state and runtime state
    let project_state = ProjectState::new();
    let runtime_state = RuntimeState::new();

    // Initialize runtime (needed for project loading)
    let _ = runtime_state.initialize().await;

    // Create a mock project manually
    let mock_project = Project {
        name: "Test Project".to_string(),
        root_path: "/mock/path/test-project".to_string(),
        folders: vec!["src".to_string(), "tests".to_string()],
        is_loaded: true,
    };

    // Set the mock project
    let mut current_project = project_state.current_project.lock().await;
    *current_project = Some(mock_project.clone());
    drop(current_project);

    // Check that the project was set
    let retrieved_project = project_state.get_current_project().await;
    assert!(retrieved_project.is_some());

    let retrieved_project = retrieved_project.unwrap();
    assert_eq!(retrieved_project.name, "Test Project");
    assert_eq!(retrieved_project.root_path, "/mock/path/test-project");
    assert_eq!(retrieved_project.folders.len(), 2);
    assert!(retrieved_project.is_loaded);

    // Clean up
    let _ = project_state.close_project().await;
    let _ = runtime_state.shutdown().await;
}

// Test utility function
#[test]
fn test_mock_gather_folders() {
    let path = PathBuf::from("/mock/path");
    let folders = mock_gather_folders(&path).unwrap();

    assert_eq!(folders.len(), 3);
    assert!(folders.contains(&"src".to_string()));
    assert!(folders.contains(&"tests".to_string()));
    assert!(folders.contains(&"docs".to_string()));
}

#[tokio::test]
async fn test_has_project() {
    // Create a new project state
    let project_state = ProjectState::new();

    // Initially, has_project should return false
    assert!(!project_state.has_project().await);

    // Create a test project
    let test_project = Project {
        name: "Test Project".to_string(),
        root_path: "/path/to/test".to_string(),
        folders: vec!["src".to_string(), "tests".to_string()],
        is_loaded: true,
    };

    // Set the project manually
    {
        let mut current_project = project_state.current_project.lock().await;
        *current_project = Some(test_project);
    }

    // Now has_project should return true
    assert!(project_state.has_project().await);
}
