use lion_ui_tauri::agents::{Agent, AgentManager, AgentState};
use lion_ui_tauri::runtime::RuntimeState;

#[tokio::test]
async fn test_agent_state_display() {
    // Test each variant of AgentState implements Display correctly
    assert_eq!(AgentState::Stopped.to_string(), "Stopped");
    assert_eq!(AgentState::Starting.to_string(), "Starting");
    assert_eq!(AgentState::Running.to_string(), "Running");
    assert_eq!(AgentState::Stopping.to_string(), "Stopping");
    assert_eq!(AgentState::Error.to_string(), "Error");
}

#[tokio::test]
async fn test_agent_manager_add_and_get() {
    // Create a new agent manager
    let agent_manager = AgentManager::new();

    // Initially, there should be no agents
    let initial_agents = agent_manager.get_agents().await;
    assert_eq!(initial_agents.len(), 0);

    // Create and add a test agent
    let test_agent = Agent {
        id: "test1".to_string(),
        name: "Test Agent".to_string(),
        agent_type: "test".to_string(),
        description: "Test agent for unit testing".to_string(),
        state: AgentState::Stopped,
        capabilities: vec!["testing".to_string()],
    };

    // Add the agent
    agent_manager.add_agent(test_agent.clone()).await;

    // Check that the agent was added
    let agents = agent_manager.get_agents().await;
    assert_eq!(agents.len(), 1);

    // Check that we can get a specific agent
    let retrieved_agent = agent_manager.get_agent("test1").await;
    assert!(retrieved_agent.is_some());

    let retrieved_agent = retrieved_agent.unwrap();
    assert_eq!(retrieved_agent.id, "test1");
    assert_eq!(retrieved_agent.name, "Test Agent");
    assert_eq!(retrieved_agent.state, AgentState::Stopped);

    // Check getting a non-existent agent
    let non_existent = agent_manager.get_agent("does-not-exist").await;
    assert!(non_existent.is_none());
}

#[tokio::test]
async fn test_agent_state_updates() {
    // Create a new agent manager
    let agent_manager = AgentManager::new();

    // Add a test agent
    let test_agent = Agent {
        id: "test2".to_string(),
        name: "State Test Agent".to_string(),
        agent_type: "test".to_string(),
        description: "Testing state changes".to_string(),
        state: AgentState::Stopped,
        capabilities: vec!["state-testing".to_string()],
    };

    agent_manager.add_agent(test_agent).await;

    // Update state to STARTING
    let update_result = agent_manager
        .update_agent_state("test2", AgentState::Starting)
        .await;
    assert!(update_result.is_ok());

    // Check state was updated
    let agent = agent_manager.get_agent("test2").await.unwrap();
    assert_eq!(agent.state, AgentState::Starting);

    // Update state to RUNNING
    let update_result = agent_manager
        .update_agent_state("test2", AgentState::Running)
        .await;
    assert!(update_result.is_ok());

    // Check state was updated
    let agent = agent_manager.get_agent("test2").await.unwrap();
    assert_eq!(agent.state, AgentState::Running);

    // Try to update a non-existent agent
    let update_result = agent_manager
        .update_agent_state("does-not-exist", AgentState::Running)
        .await;
    assert!(update_result.is_err());
}

#[tokio::test]
async fn test_load_agents_from_runtime() {
    // Create a new agent manager and runtime state
    let agent_manager = AgentManager::new();
    let runtime_state = RuntimeState::new();

    // Initialize the runtime
    let _ = runtime_state.initialize().await;

    // Load agents from runtime (mock implementation that adds sample agents)
    let load_result = agent_manager.load_agents_from_runtime(&runtime_state).await;
    assert!(load_result.is_ok());

    // Check that agents were loaded
    let agents = agent_manager.get_agents().await;
    assert!(!agents.is_empty());

    // In our mock implementation, we should have at least the sample agents
    assert!(agents.len() >= 3);

    // Verify some of the loaded agents
    let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
    assert!(agent_names.contains(&"System Monitor".to_string()));
    assert!(agent_names.contains(&"Data Processor".to_string()));
    assert!(agent_names.contains(&"API Gateway".to_string()));

    // Clean up
    let _ = runtime_state.shutdown().await;
}

#[tokio::test]
async fn test_remove_agent() {
    // Create a new agent manager
    let agent_manager = AgentManager::new();

    // Add a test agent
    let test_agent = Agent {
        id: "test-remove".to_string(),
        name: "Agent to Remove".to_string(),
        agent_type: "test".to_string(),
        description: "Testing agent removal".to_string(),
        state: AgentState::Running,
        capabilities: vec!["removal-testing".to_string()],
    };

    agent_manager.add_agent(test_agent).await;

    // Verify agent was added
    let agent = agent_manager.get_agent("test-remove").await;
    assert!(agent.is_some());

    // Remove the agent
    let remove_result = agent_manager.remove_agent("test-remove").await;
    assert!(remove_result.is_ok());

    // Verify agent was removed
    let agent = agent_manager.get_agent("test-remove").await;
    assert!(agent.is_none());

    // Try to remove a non-existent agent
    let remove_result = agent_manager.remove_agent("does-not-exist").await;
    assert!(remove_result.is_err());
}
