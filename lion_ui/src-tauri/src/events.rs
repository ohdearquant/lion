use std::sync::Arc;
use tauri::{Emitter, Manager, WebviewWindow};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::agents::{Agent, AgentManager, AgentState};
use crate::logging::{add_log, LogBuffer, LogLevel};
use crate::runtime::{RuntimeState, RuntimeStatus};

// Struct to manage event emission
pub struct EventManager {
    pub window: Arc<Mutex<Option<WebviewWindow>>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            window: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_window(&self, window: WebviewWindow) {
        let mut window_lock = self.window.lock().await;
        *window_lock = Some(window);
    }

    pub async fn get_window(&self) -> Option<WebviewWindow> {
        self.window.lock().await.clone()
    }

    pub async fn emit_runtime_status(&self, status: RuntimeStatus) -> Result<(), String> {
        if let Some(window) = &*self.window.lock().await {
            window
                .emit_to(window.label(), "runtime_status_changed", status)
                .map_err(|e| format!("Failed to emit runtime status event: {}", e))
        } else {
            Err("Window not set".to_string())
        }
    }

    pub async fn emit_agent_status(
        &self,
        id: &str,
        name: &str,
        new_state: &str,
    ) -> Result<(), String> {
        if let Some(window) = &*self.window.lock().await {
            window
                .emit_to(
                    window.label(),
                    "agent_status_changed",
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "new_state": new_state
                    }),
                )
                .map_err(|e| format!("Failed to emit agent status event: {}", e))
        } else {
            Err("Window not set".to_string())
        }
    }
}

// Start a background task to periodically update and emit the runtime status
pub fn start_runtime_status_updater(
    app_handle: tauri::AppHandle,
    runtime_state: RuntimeState,
    event_manager: EventManager,
    log_buffer: LogBuffer,
) {
    // Clone necessary state for the background task
    let app_handle_clone = app_handle.clone();

    // Spawn a Tokio task for periodic status updates
    tauri::async_runtime::spawn(async move {
        loop {
            // Update and emit runtime status every 1 second
            match runtime_state.update_status().await {
                Ok(status) => {
                    // If the status was updated successfully, emit an event
                    if let Err(e) = event_manager.emit_runtime_status(status.clone()).await {
                        eprintln!("Error emitting runtime status: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Error updating runtime status: {}", e);

                    // Log the error
                    if let Some(window) = app_handle_clone.get_webview_window("main") {
                        add_log(
                            crate::logging::LogLevel::Error,
                            "Runtime",
                            format!("Failed to update runtime status: {}", e),
                            &log_buffer,
                            Some(&window),
                        )
                        .await;
                    }
                }
            }

            // Sleep for 1 second before the next update
            sleep(Duration::from_secs(1)).await;
        }
    });
}

// Simulate periodic agent state changes (for testing)
pub fn simulate_agent_state_changes(
    app_handle: tauri::AppHandle,
    agent_manager: AgentManager,
    event_manager: EventManager,
    log_buffer: LogBuffer,
) {
    // Clone necessary state for the background task
    let app_handle_clone = app_handle.clone();

    // Spawn a Tokio task for simulating agent state changes
    tauri::async_runtime::spawn(async move {
        // Wait a few seconds to start
        sleep(Duration::from_secs(5)).await;

        let mut toggle = true;

        loop {
            // Get current agents
            let agents = agent_manager.get_agents().await;

            if let Some(agent) = agents.first() {
                // Alternate between Running and Stopped for the first agent
                let new_state = if toggle {
                    crate::agents::AgentState::Running
                } else {
                    crate::agents::AgentState::Stopped
                };

                // Update agent state
                if let Err(e) = agent_manager
                    .update_agent_state(&agent.id, new_state.clone())
                    .await
                {
                    eprintln!("Error updating agent state: {}", e);
                } else {
                    // Emit agent status change event
                    if let Err(e) = event_manager
                        .emit_agent_status(&agent.id, &agent.name, &new_state.to_string())
                        .await
                    {
                        eprintln!("Error emitting agent status: {}", e);
                    }

                    // Log the state change
                    if let Some(window) = app_handle_clone.get_webview_window("main") {
                        add_log(
                            crate::logging::LogLevel::Info,
                            "AgentManager",
                            format!("Agent '{}' state changed to {}", agent.name, new_state),
                            &log_buffer,
                            Some(&window),
                        )
                        .await;
                    }
                }

                toggle = !toggle;
            }

            // Sleep for 10 seconds before the next change
            sleep(Duration::from_secs(10)).await;
        }
    });
}

// Simulate periodic log entries (for testing)
pub fn simulate_log_entries(app_handle: tauri::AppHandle, log_buffer: LogBuffer) {
    // Spawn a Tokio task for simulating log entries
    tauri::async_runtime::spawn(async move {
        let log_messages = [
            "System startup complete",
            "Processing input data",
            "API request received",
            "Database connection established",
            "Cache refreshed",
            "Task scheduled",
            "User session started",
            "Configuration loaded",
            "Resources initialized",
            "Metrics collected",
        ];

        let log_sources = [
            "System",
            "DataProcessor",
            "APIGateway",
            "Database",
            "CacheManager",
            "Scheduler",
            "UserService",
            "ConfigLoader",
            "ResourceManager",
            "MetricsCollector",
        ];

        let log_levels = [
            crate::logging::LogLevel::Info,
            crate::logging::LogLevel::Debug,
            crate::logging::LogLevel::Warning,
            crate::logging::LogLevel::Error,
        ];

        let mut counter = 0;

        loop {
            // Select a random log message, source, and level
            let message = log_messages[counter % log_messages.len()];
            let source = log_sources[counter % log_sources.len()];
            let level = &log_levels[counter % log_levels.len()];

            // Get the main window
            if let Some(window) = app_handle.get_webview_window("main") {
                // Add the log entry
                add_log(
                    level.clone(),
                    source,
                    format!("{} (#{}))", message, counter),
                    &log_buffer,
                    Some(&window),
                )
                .await;
            }

            counter += 1;

            // Sleep for a random duration between 2-5 seconds
            let sleep_duration = Duration::from_secs(2 + (counter % 4) as u64);
            sleep(sleep_duration).await;
        }
    });
}
