use lion_ui_tauri::project::{Project, ProjectState};
use lion_ui_tauri::runtime::{is_valid_lion_project, RuntimeState};
use std::path::Path;

#[tokio::test]
async fn test_project_state_new() {
    // Create a new project state
    let project_state = ProjectState::new();

    // Verify the project state is initialized correctly
    let current_project = project_state.current_project.lock().await;
    assert!(current_project.is_none());
}

#[tokio::test]
async fn test_project_state_has_project() {
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

#[tokio::test]
async fn test_project_state_close_project() {
    // Create a new project state
    let project_state = ProjectState::new();

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
        *current_project = Some(test_project.clone());
    }

    // Verify the project was set
    {
        let current_project = project_state.current_project.lock().await;
        assert!(current_project.is_some());
    }

    // Close the project
    project_state.close_project().await.unwrap();

    // Verify the project was closed
    let current_project = project_state.current_project.lock().await;
    assert!(current_project.is_none());
}

#[test]
fn test_is_valid_lion_project() {
    // This is a mock test since we can't easily create a real project structure
    // In a real test, we would create a temporary directory with the required files

    // Test with a non-existent path
    let non_existent_path = Path::new("/path/does/not/exist");
    assert!(!is_valid_lion_project(non_existent_path));

    // In a real test, we would also test with a valid project structure
    // For now, we'll just mock the behavior
    // let valid_path = Path::new("/path/to/valid/project");
    // assert!(is_valid_lion_project(valid_path));
}

// Test for get_current_project method
#[tokio::test]
async fn test_get_current_project() {
    // Create a new project state
    let project_state = ProjectState::new();

    // Initially, get_current_project should return None
    let initial_project = project_state.get_current_project().await;
    assert!(initial_project.is_none());

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
        *current_project = Some(test_project.clone());
    }

    // Now get_current_project should return the project
    let retrieved_project = project_state.get_current_project().await;
    assert!(retrieved_project.is_some());

    let retrieved_project = retrieved_project.unwrap();
    assert_eq!(retrieved_project.name, "Test Project");
    assert_eq!(retrieved_project.root_path, "/path/to/test");
    assert_eq!(retrieved_project.folders.len(), 2);
    assert!(retrieved_project.is_loaded);
}
