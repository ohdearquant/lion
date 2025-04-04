use crate::agents::{Agent, AgentState};
use crate::logging::{LogEntry, LogLevel};
use crate::project::{identify_project_internal, open_project_internal, Project};
use crate::runtime::{get_runtime_status_internal, RuntimeStatus};
use crate::state::AppState;
use tauri::{command, State};

/// Get the current runtime status
#[command]
pub async fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    get_runtime_status_internal(&state.runtime_state).await
}

/// Get a list of all agents
#[command]
pub async fn list_agents(state: State<'_, AppState>) -> Result<Vec<Agent>, String> {
    Ok(state.agent_manager.get_agents().await)
}

/// Get recent log entries
#[command]
pub async fn get_recent_logs(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<LogEntry>, String> {
    let limit = limit.unwrap_or(100);
    Ok(state.log_buffer.get_recent_logs(limit).await)
}

/// Clear all logs
#[command]
pub async fn clear_logs(state: State<'_, AppState>) -> Result<(), String> {
    state.log_buffer.clear_logs().await;

    // Add a log entry about clearing logs
    if let Some(window) = state.event_manager.get_window().await {
        crate::logging::add_log(
            LogLevel::Info,
            "system",
            "Logs cleared",
            &state.log_buffer,
            Some(&window),
        )
        .await;
    }

    Ok(())
}

/// Identify a project at the given path
#[command]
pub async fn identify_project(
    state: State<'_, AppState>,
    path: String,
) -> Result<(String, bool), String> {
    // Log the attempt
    if let Some(window) = state.event_manager.get_window().await {
        crate::logging::add_log(
            LogLevel::Info,
            "project",
            format!("Identifying project at path: {}", path),
            &state.log_buffer,
            Some(&window),
        )
        .await;
    }

    let result = identify_project_internal(path.clone()).await;

    // Log the result
    if let Some(window) = state.event_manager.get_window().await {
        match &result {
            Ok((name, valid)) => {
                if *valid {
                    crate::logging::add_log(
                        LogLevel::Info,
                        "project",
                        format!("Identified valid project: {} at {}", name, path),
                        &state.log_buffer,
                        Some(&window),
                    )
                    .await;
                } else {
                    crate::logging::add_log(
                        LogLevel::Warning,
                        "project",
                        format!("Invalid project at path: {}", path),
                        &state.log_buffer,
                        Some(&window),
                    )
                    .await;
                }
            }
            Err(e) => {
                crate::logging::add_log(
                    LogLevel::Error,
                    "project",
                    format!("Failed to identify project at {}: {}", path, e),
                    &state.log_buffer,
                    Some(&window),
                )
                .await;
            }
        }
    }

    result
}

/// Open a project
#[command]
pub async fn open_project(state: State<'_, AppState>, path: String) -> Result<Project, String> {
    // Log the attempt
    if let Some(window) = state.event_manager.get_window().await {
        crate::logging::add_log(
            LogLevel::Info,
            "project",
            format!("Opening project at path: {}", path),
            &state.log_buffer,
            Some(&window),
        )
        .await;
    }

    let result =
        open_project_internal(path.clone(), &state.project_state, &state.runtime_state).await;

    // Log the result
    if let Some(window) = state.event_manager.get_window().await {
        match &result {
            Ok(project) => {
                crate::logging::add_log(
                    LogLevel::Info,
                    "project",
                    format!("Opened project: {} at {}", project.name, path),
                    &state.log_buffer,
                    Some(&window),
                )
                .await;
            }
            Err(e) => {
                crate::logging::add_log(
                    LogLevel::Error,
                    "project",
                    format!("Failed to open project at {}: {}", path, e),
                    &state.log_buffer,
                    Some(&window),
                )
                .await;
            }
        }
    }

    result
}

/// Close the current project
#[command]
pub async fn close_project(state: State<'_, AppState>) -> Result<(), String> {
    // Get the current project name for logging
    let project_name = if let Some(project) = state.project_state.get_current_project().await {
        project.name.clone()
    } else {
        "Unknown".to_string()
    };

    // Log the action
    if let Some(window) = state.event_manager.get_window().await {
        if state.project_state.has_project().await {
            crate::logging::add_log(
                LogLevel::Info,
                "project",
                format!("Closing project: {}", project_name),
                &state.log_buffer,
                Some(&window),
            )
            .await;
        } else {
            crate::logging::add_log(
                LogLevel::Warning,
                "project",
                "No project is currently open",
                &state.log_buffer,
                Some(&window),
            )
            .await;
        }
    }

    // Close the project
    let _ = state.project_state.close_project().await;

    Ok(())
}

/// Update an agent's state
#[command]
pub async fn update_agent_state(
    state: State<'_, AppState>,
    agent_id: String,
    new_state: String,
) -> Result<(), String> {
    // Log the attempt
    if let Some(window) = state.event_manager.get_window().await {
        crate::logging::add_log(
            LogLevel::Info,
            "agents",
            format!("Updating agent {} state to {}", agent_id, new_state),
            &state.log_buffer,
            Some(&window),
        )
        .await;
    }

    // Parse the new state string to AgentState
    let agent_state = match new_state.as_str() {
        "Stopped" => AgentState::Stopped,
        "Starting" => AgentState::Starting,
        "Running" => AgentState::Running,
        "Stopping" => AgentState::Stopping,
        "Error" => AgentState::Error,
        _ => AgentState::Stopped, // Default to Stopped for unknown states
    };

    // Update the agent state
    let result = state
        .agent_manager
        .update_agent_state(&agent_id, agent_state)
        .await;

    // Log the result
    if let Err(e) = &result {
        if let Some(window) = state.event_manager.get_window().await {
            crate::logging::add_log(
                LogLevel::Error,
                "agents",
                format!("Failed to update agent state: {}", e),
                &state.log_buffer,
                Some(&window),
            )
            .await;
        }
    }

    result
}
