use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

use crate::runtime::{is_valid_lion_project, RuntimeState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub root_path: String,
    pub folders: Vec<String>,
    pub is_loaded: bool,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: String::new(),
            root_path: String::new(),
            folders: Vec::new(),
            is_loaded: false,
        }
    }
}

#[derive(Default)]
pub struct ProjectState {
    pub current_project: Arc<Mutex<Option<Project>>>,
}

impl ProjectState {
    pub fn new() -> Self {
        Self {
            current_project: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn load_project(
        &self,
        path: &Path,
        runtime_state: &RuntimeState,
    ) -> Result<Project, String> {
        if !is_valid_lion_project(path) {
            return Err(format!("Invalid Lion project at path: {}", path.display()));
        }

        // Create project data
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown Project")
            .to_string();

        // Gather folder information
        let folders = gather_project_folders(path)?;

        let project = Project {
            name,
            root_path: path.to_string_lossy().to_string(),
            folders,
            is_loaded: false, // Will be set to true after runtime loads it
        };

        // Store the current project
        let mut current_project = self.current_project.lock().await;
        *current_project = Some(project.clone());

        // Here we would load the project into the runtime
        // This is a placeholder for actual runtime project loading
        if runtime_state.is_runtime_initialized().await {
            // For now, we'll just mark the project as loaded
            let mut current_project = self.current_project.lock().await;
            if let Some(project) = current_project.as_mut() {
                project.is_loaded = true;
            }
        }

        Ok(project)
    }

    pub async fn get_current_project(&self) -> Option<Project> {
        self.current_project.lock().await.clone()
    }

    pub async fn has_project(&self) -> bool {
        self.current_project.lock().await.is_some()
    }

    pub async fn close_project(&self) -> Result<(), String> {
        let mut current_project = self.current_project.lock().await;
        *current_project = None;
        Ok(())
    }
}

// Utility function to gather folder information from a project
fn gather_project_folders(path: &Path) -> Result<Vec<String>, String> {
    let mut folders = Vec::new();

    // Read the directory entries
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => return Err(format!("Failed to read project directory: {}", e)),
    };

    // Process each entry
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("Error reading directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if path.is_dir() {
            // Check if this is a directory we want to include
            // Skip hidden directories (those starting with .)
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    if !file_name_str.starts_with('.') && !file_name_str.eq("target") {
                        folders.push(file_name_str.to_string());
                    }
                }
            }
        }
    }

    // Sort folders alphabetically for consistency
    folders.sort();

    Ok(folders)
}

pub async fn identify_project_internal(path: String) -> Result<(String, bool), String> {
    let path = PathBuf::from(path);

    // Check if the path is a valid Lion project
    let valid = is_valid_lion_project(&path);

    // Get the project name (last directory name)
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Unknown Project")
        .to_string();

    Ok((name, valid))
}

pub async fn open_project_internal(
    path: String,
    project_state: &ProjectState,
    runtime_state: &RuntimeState,
) -> Result<Project, String> {
    let path = PathBuf::from(path);
    project_state.load_project(&path, runtime_state).await
}
