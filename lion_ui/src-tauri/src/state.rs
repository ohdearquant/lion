use tauri::{AppHandle, Manager, State};

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agents::AgentManager;
use crate::events::{
    simulate_agent_state_changes, simulate_log_entries, start_runtime_status_updater, EventManager,
};
use crate::logging::{add_log, LogBuffer, LogLevel};
use crate::project::ProjectState;
use crate::runtime::RuntimeState;
use crate::workflows::WorkflowDefinition;

// Define a simple WorkflowManager struct since the one in workflows.rs is not available
pub struct WorkflowManager {
    pub definitions: Arc<Mutex<Vec<WorkflowDefinition>>>,
    pub instances: Arc<Mutex<Vec<String>>>,
}

impl WorkflowManager {
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(Mutex::new(Vec::new())),
            instances: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

// State accessor functions - return clones to avoid lifetime issues
pub fn get_runtime(state: State<'_, AppState>) -> RuntimeState {
    state.runtime_state.clone()
}

pub fn get_log_buffer(state: State<'_, AppState>) -> LogBuffer {
    state.log_buffer.clone()
}

pub fn get_project_state(state: State<'_, AppState>) -> ProjectState {
    state.project_state.clone()
}

pub fn get_agent_registry(state: State<'_, AppState>) -> AgentManager {
    state.agent_manager.clone()
}

pub fn get_workflow_manager(state: State<'_, AppState>) -> WorkflowManager {
    state.workflow_manager.clone()
}
/// Application state container
pub struct AppState {
    pub runtime_state: RuntimeState,
    pub log_buffer: LogBuffer,
    pub project_state: ProjectState,
    pub agent_manager: AgentManager,
    pub workflow_manager: WorkflowManager, // Added WorkflowManager field
    pub event_manager: EventManager,
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            runtime_state: RuntimeState::new(),
            log_buffer: LogBuffer::new(1000), // Store up to 1000 log entries
            project_state: ProjectState::new(),
            agent_manager: AgentManager::new(),
            workflow_manager: WorkflowManager::new(), // Initialize WorkflowManager
            event_manager: EventManager::new(),
        }
    }

    /// Initialize the application state with the app handle
    pub async fn initialize(&self, app_handle: AppHandle) {
        // Set the window for event emission
        if let Some(main_window) = app_handle.get_webview_window("main") {
            self.event_manager.set_window(main_window.clone()).await;

            // Add initial log entry
            add_log(
                crate::logging::LogLevel::Info,
                "System",
                "LionForge IDE initialized",
                &self.log_buffer,
                Some(&main_window),
            )
            .await;
        }

        // Initialize runtime
        match self.runtime_state.initialize().await {
            Ok(_) => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    add_log(
                        crate::logging::LogLevel::Info,
                        "Runtime",
                        "Runtime initialized successfully",
                        &self.log_buffer,
                        Some(&window),
                    )
                    .await;
                }
            }
            Err(e) => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    add_log(
                        crate::logging::LogLevel::Error,
                        "Runtime",
                        format!("Failed to initialize runtime: {}", e),
                        &self.log_buffer,
                        Some(&window),
                    )
                    .await;
                }
            }
        }

        // Initialize agent manager
        match self
            .agent_manager
            .load_agents_from_runtime(&self.runtime_state)
            .await
        {
            Ok(_) => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let agent_count = self.agent_manager.get_agents().await.len();
                    add_log(
                        crate::logging::LogLevel::Info,
                        "Agents",
                        format!("Initialized {} agents", agent_count),
                        &self.log_buffer,
                        Some(&window),
                    )
                    .await;
                }
            }
            Err(e) => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    add_log(
                        crate::logging::LogLevel::Error,
                        "Agents",
                        format!("Failed to load agents: {}", e),
                        &self.log_buffer,
                        Some(&window),
                    )
                    .await;
                }
            }
        }

        // Initialize workflow manager (basic log for now)
        if let Some(window) = app_handle.get_webview_window("main") {
            add_log(
                crate::logging::LogLevel::Info,
                "Workflows",
                "Workflow Manager initialized",
                &self.log_buffer,
                Some(&window),
            )
            .await;
        }

        // Log successful initialization
        if let Some(window) = app_handle.get_webview_window("main") {
            add_log(
                crate::logging::LogLevel::Info,
                "System",
                "All subsystems initialized successfully",
                &self.log_buffer,
                Some(&window),
            )
            .await;
        }

        // Start background tasks
        start_runtime_status_updater(
            app_handle.clone(),
            self.runtime_state.clone(),
            self.event_manager.clone(),
            self.log_buffer.clone(),
        );

        // Start simulation tasks (for demo purposes)
        simulate_agent_state_changes(
            app_handle.clone(),
            self.agent_manager.clone(),
            self.event_manager.clone(),
            self.log_buffer.clone(),
        );

        simulate_log_entries(app_handle.clone(), self.log_buffer.clone());
    }
}

/// Create and initialize the application state
pub fn setup_state(app: &tauri::App) {
    // Use a separate function to avoid borrowing issues
    let app_handle = app.app_handle();
    initialize_state_async(app_handle.clone());
}

// Separate function to handle the async initialization
fn initialize_state_async(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<AppState>();
        state.initialize(app_handle.clone()).await;
    });
}

// Make these types cloneable for our implementation
impl Clone for RuntimeState {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            status: self.status.clone(),
            start_time: self.start_time.clone(),
        }
    }
}

impl Clone for LogBuffer {
    fn clone(&self) -> Self {
        Self {
            logs: self.logs.clone(),
            max_logs: self.max_logs.clone(),
        }
    }
}

impl Clone for ProjectState {
    fn clone(&self) -> Self {
        Self {
            current_project: self.current_project.clone(),
        }
    }
}

impl Clone for AgentManager {
    fn clone(&self) -> Self {
        Self {
            agents: self.agents.clone(),
        }
    }
}

// Add Clone impl for WorkflowManager if not already present in workflows.rs
// (Assuming it's defined similarly to AgentManager)
impl Clone for WorkflowManager {
    fn clone(&self) -> Self {
        Self {
            definitions: self.definitions.clone(),
            instances: self.instances.clone(),
        }
    }
}

impl Clone for EventManager {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
        }
    }
}
