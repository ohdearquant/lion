use std::path::Path;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// Stub implementation of Runtime for testing
pub struct Runtime {
    agent_count: usize,
}

impl Runtime {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            agent_count: 3, // Default to 3 agents for testing
        })
    }

    pub fn get_agent_count(&self) -> Result<usize, String> {
        Ok(self.agent_count)
    }
}
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub is_running: bool,
    pub version: String,
    pub uptime_seconds: u64,
    pub agent_count: usize,
    pub error: Option<String>,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            is_running: false,
            version: String::from("0.0.0"),
            uptime_seconds: 0,
            agent_count: 0,
            error: None,
        }
    }
}

#[derive(Default)]
pub struct RuntimeState {
    pub runtime: Arc<Mutex<Option<Runtime>>>,
    pub status: Arc<Mutex<RuntimeStatus>>,
    pub start_time: Arc<Mutex<Option<std::time::Instant>>>,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(RuntimeStatus::default())),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn initialize(&self) -> Result<(), String> {
        let mut runtime_lock = self.runtime.lock().await;
        if runtime_lock.is_some() {
            return Err("Runtime already initialized".to_string());
        }

        // Create a new runtime
        match Runtime::new() {
            Ok(runtime) => {
                *runtime_lock = Some(runtime);
                let mut start_time = self.start_time.lock().await;
                *start_time = Some(std::time::Instant::now());

                // Update status
                let mut status = self.status.lock().await;
                status.is_running = true;
                status.version = env!("CARGO_PKG_VERSION").to_string();
                status.agent_count = 0; // Will be updated when agents are loaded
                status.error = None;

                Ok(())
            }
            Err(e) => {
                let mut status = self.status.lock().await;
                status.is_running = false;
                status.error = Some(format!("Failed to initialize runtime: {}", e));
                Err(format!("Failed to initialize runtime: {}", e))
            }
        }
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        let mut runtime_lock = self.runtime.lock().await;
        if runtime_lock.is_none() {
            return Err("Runtime not initialized".to_string());
        }

        // Shut down the runtime
        if let Some(runtime) = runtime_lock.take() {
            // Perform any necessary cleanup operations on the runtime
            drop(runtime);

            // Update status
            let mut status = self.status.lock().await;
            status.is_running = false;

            // Reset start time
            let mut start_time = self.start_time.lock().await;
            *start_time = None;

            Ok(())
        } else {
            Err("Runtime not initialized".to_string())
        }
    }

    pub async fn update_status(&self) -> Result<RuntimeStatus, String> {
        let mut status = self.status.lock().await;

        // Update uptime if runtime is running
        if status.is_running {
            if let Some(start_time) = *self.start_time.lock().await {
                status.uptime_seconds = start_time.elapsed().as_secs();
            }

            // Update agent count if runtime exists
            if let Some(runtime) = &*self.runtime.lock().await {
                status.agent_count = runtime.get_agent_count().unwrap_or(0);
            }
        }

        Ok(status.clone())
    }

    pub async fn is_runtime_initialized(&self) -> bool {
        self.runtime.lock().await.is_some()
    }
}

// Internal function to get runtime status
pub async fn get_runtime_status_internal(
    runtime_state: &RuntimeState,
) -> Result<RuntimeStatus, String> {
    runtime_state.update_status().await
}

// Helper function to check if a path is a valid Lion project
pub fn is_valid_lion_project(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    // Check for lion.toml file
    let lion_toml = path.join("lion.toml");
    if !lion_toml.exists() {
        return false;
    }

    // Additional validation could be done here, like checking the contents
    // of the lion.toml file or verifying other required project structure

    true
}
