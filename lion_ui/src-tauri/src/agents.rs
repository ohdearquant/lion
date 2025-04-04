use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::logging::LogLevel;
use crate::runtime::RuntimeState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentState::Stopped => write!(f, "Stopped"),
            AgentState::Starting => write!(f, "Starting"),
            AgentState::Running => write!(f, "Running"),
            AgentState::Stopping => write!(f, "Stopping"),
            AgentState::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub description: String,
    pub state: AgentState,
    pub capabilities: Vec<String>,
}

#[derive(Default)]
pub struct AgentManager {
    pub agents: Arc<Mutex<Vec<Agent>>>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add_agent(&self, agent: Agent) {
        let mut agents = self.agents.lock().await;
        agents.push(agent);
    }

    pub async fn update_agent_state(&self, id: &str, new_state: AgentState) -> Result<(), String> {
        let mut agents = self.agents.lock().await;

        if let Some(agent) = agents.iter_mut().find(|a| a.id == id) {
            agent.state = new_state;
            Ok(())
        } else {
            Err(format!("Agent with ID {} not found", id))
        }
    }

    pub async fn get_agents(&self) -> Vec<Agent> {
        self.agents.lock().await.clone()
    }

    pub async fn get_agent(&self, id: &str) -> Option<Agent> {
        let agents = self.agents.lock().await;
        agents.iter().find(|a| a.id == id).cloned()
    }

    pub async fn remove_agent(&self, id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().await;

        let initial_len = agents.len();
        agents.retain(|a| a.id != id);

        if agents.len() < initial_len {
            Ok(())
        } else {
            Err(format!("Agent with ID {} not found", id))
        }
    }

    // This would be called to load agents from the runtime
    pub async fn load_agents_from_runtime(
        &self,
        runtime_state: &RuntimeState,
    ) -> Result<(), String> {
        let runtime_lock = runtime_state.runtime.lock().await;

        if let Some(_runtime) = &*runtime_lock {
            // In a real implementation, we would query the runtime for agents
            // For now, we'll add a few mock agents

            let mut agents = self.agents.lock().await;
            agents.clear();

            // Add some example agents
            agents.push(Agent {
                id: "agent1".to_string(),
                name: "System Monitor".to_string(),
                agent_type: "system".to_string(),
                description: "Monitors system resources and performance".to_string(),
                state: AgentState::Running,
                capabilities: vec!["monitoring".to_string(), "metrics".to_string()],
            });

            agents.push(Agent {
                id: "agent2".to_string(),
                name: "Data Processor".to_string(),
                agent_type: "processor".to_string(),
                description: "Processes and transforms data".to_string(),
                state: AgentState::Stopped,
                capabilities: vec!["processing".to_string(), "transformation".to_string()],
            });

            agents.push(Agent {
                id: "agent3".to_string(),
                name: "API Gateway".to_string(),
                agent_type: "gateway".to_string(),
                description: "Handles external API requests".to_string(),
                state: AgentState::Running,
                capabilities: vec!["api".to_string(), "gateway".to_string()],
            });

            Ok(())
        } else {
            Err("Runtime not initialized".to_string())
        }
    }

    // Helper method to emit agent state change events
    pub async fn emit_agent_state_change(
        &self,
        app_handle: &tauri::AppHandle,
        agent_id: &str,
        agent_name: &str,
        new_state: AgentState,
    ) -> Result<(), String> {
        // Create the event payload
        let payload = serde_json::json!({
            "id": agent_id,
            "name": agent_name,
            "new_state": new_state.to_string(),
        });

        // Emit the event to the main window
        if let Some(window) = app_handle.get_webview_window("main") {
            window
                .emit_to(window.label(), "agent_status_changed", payload)
                .map_err(|e| format!("Failed to emit agent status event: {}", e))?;
        }

        // Log the state change
        if let Some(window) = app_handle.get_webview_window("main") {
            crate::logging::add_log(
                LogLevel::Info,
                "Agents",
                format!("Agent {} state changed to {}", agent_name, new_state),
                &app_handle.state::<crate::logging::LogBuffer>(),
                Some(&window),
            )
            .await;
        }

        Ok(())
    }
}

/// Load an agent from a file (WASM or configuration)
#[tauri::command]
pub async fn load_agent(
    path: String,
    agent_manager: State<'_, AgentManager>,
    _runtime_state: State<'_, crate::runtime::RuntimeState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // Determine if path is WASM or config file
    let file_extension = Path::new(&path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match file_extension {
        "wasm" => {
            // Load WASM file directly
            let _wasm_bytes =
                std::fs::read(&path).map_err(|e| format!("Failed to read WASM file: {}", e))?;

            // Extract filename for agent name
            let filename = Path::new(&path)
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown_agent")
                .to_string();

            // In a real implementation, we would call runtime.plugins.load_plugin()
            // For now, we'll create a mock agent

            // Generate a unique ID
            let agent_id = format!("agent-{}", uuid::Uuid::new_v4());

            // Create the agent
            let agent = Agent {
                id: agent_id.clone(),
                name: filename.clone(),
                agent_type: "wasm".to_string(),
                description: format!("Loaded from {}", path),
                state: AgentState::Starting,
                capabilities: vec![],
            };

            // Add the agent
            agent_manager.add_agent(agent.clone()).await;

            // Emit initial state event
            agent_manager
                .emit_agent_state_change(&app_handle, &agent_id, &filename, AgentState::Starting)
                .await?;

            // Add a log entry for loading
            if let Some(window) = app_handle.get_webview_window("main") {
                crate::logging::add_log(
                    LogLevel::Info,
                    "Agents",
                    format!("Loading agent {} from {}", filename, path),
                    &app_handle.state::<crate::logging::LogBuffer>(),
                    Some(&window),
                )
                .await;
            }

            // Simulate agent initialization
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Update the agent state to RUNNING
            agent_manager
                .update_agent_state(&agent_id, AgentState::Running)
                .await
                .map_err(|e| format!("Failed to update agent state: {}", e))?;

            // Emit state change event
            agent_manager
                .emit_agent_state_change(&app_handle, &agent_id, &filename, AgentState::Running)
                .await?;

            Ok(agent_id)
        }
        "json" | "toml" => {
            // Parse agent configuration file
            // Implementation depends on configuration format
            Err("Agent configuration files not yet supported".to_string())
        }
        _ => Err(format!("Unsupported file type: {}", file_extension)),
    }
}

/// Unload an agent by ID
#[tauri::command]
pub async fn unload_agent(
    agent_id: String,
    agent_manager: State<'_, AgentManager>,
    _runtime_state: State<'_, crate::runtime::RuntimeState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Check if agent exists
    let agent = agent_manager
        .get_agent(&agent_id)
        .await
        .ok_or_else(|| format!("Agent not found: {}", agent_id))?;

    // Update agent state to STOPPING
    agent_manager
        .update_agent_state(&agent_id, AgentState::Stopping)
        .await?;

    // Emit state change event
    agent_manager
        .emit_agent_state_change(&app_handle, &agent_id, &agent.name, AgentState::Stopping)
        .await?;

    // In a real implementation, we would call runtime.plugins.unload_plugin()
    // For now, we'll just remove the agent from our list

    // Simulate a delay for unloading
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Remove the agent
    agent_manager.remove_agent(&agent_id).await?;

    // Add a log entry
    if let Some(window) = app_handle.get_webview_window("main") {
        crate::logging::add_log(
            LogLevel::Info,
            "Agents",
            format!("Unloaded agent {}", agent.name),
            &app_handle.state::<crate::logging::LogBuffer>(),
            Some(&window),
        )
        .await;
    }

    Ok(())
}

/// Update an agent's state
#[tauri::command]
pub async fn update_agent_state_command(
    agent_id: String,
    new_state_str: String,
    agent_manager: State<'_, AgentManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Parse the new state string to AgentState
    let new_state = match new_state_str.as_str() {
        "Stopped" => AgentState::Stopped,
        "Starting" => AgentState::Starting,
        "Running" => AgentState::Running,
        "Stopping" => AgentState::Stopping,
        "Error" => AgentState::Error,
        _ => return Err(format!("Invalid agent state: {}", new_state_str)),
    };

    // Get the agent to retrieve its name
    let agent = agent_manager
        .get_agent(&agent_id)
        .await
        .ok_or_else(|| format!("Agent not found: {}", agent_id))?;

    // Update the agent state
    agent_manager
        .update_agent_state(&agent_id, new_state.clone())
        .await?;

    // Emit state change event
    agent_manager
        .emit_agent_state_change(&app_handle, &agent_id, &agent.name, new_state)
        .await?;

    Ok(())
}
