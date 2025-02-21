use axum::{
    extract::{Multipart, Path as AxumPath, State},
    response::Json,
    routing::post,
    Router,
};
use lion_core::plugin_manager::{PluginManager, PluginManifest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path as StdPath;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

use crate::events::AppState;

/// Information about a loaded plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
}

impl From<&PluginManifest> for PluginInfo {
    fn from(manifest: &PluginManifest) -> Self {
        Self {
            id: Uuid::new_v4(), // This will be replaced with the actual ID after loading
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InvokePluginRequest {
    function: String,
    args: serde_json::Value,
}

/// Handler for loading a plugin from a manifest file
pub async fn load_plugin_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<PluginInfo> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name().unwrap() == "manifest" {
            let content = field.bytes().await.unwrap();
            let content = String::from_utf8(content.to_vec()).unwrap();
            let manifest: PluginManifest = toml::from_str(&content).unwrap();

            let mut plugin_manager = PluginManager::with_manifest_dir("plugins");
            let plugin_id = plugin_manager.load_plugin(manifest.clone()).unwrap();

            let mut info = PluginInfo::from(&manifest);
            info.id = plugin_id;

            state.plugins.write().await.insert(plugin_id, info.clone());

            return Json(info);
        }
    }
    panic!("No manifest field found in request");
}

/// Handler for listing all loaded plugins
pub async fn list_plugins_handler(State(state): State<Arc<AppState>>) -> Json<Vec<PluginInfo>> {
    let plugins = state.plugins.read().await;
    Json(plugins.values().cloned().collect())
}

/// Handler for invoking a plugin function
pub async fn invoke_plugin_handler(
    State(_state): State<Arc<AppState>>,
    AxumPath(plugin_id): AxumPath<Uuid>,
    Json(request): Json<InvokePluginRequest>,
) -> Json<serde_json::Value> {
    let input = serde_json::json!({
        "function": request.function,
        "args": request.args,
    });

    let plugin_manager = PluginManager::with_manifest_dir("plugins");
    let result = plugin_manager
        .invoke_plugin(plugin_id, &input.to_string())
        .unwrap();

    Json(serde_json::from_str(&result).unwrap())
}

/// Create router for plugin endpoints
pub fn create_plugin_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/plugins",
            post(load_plugin_handler).get(list_plugins_handler),
        )
        .route("/plugins/{plugin_id}/invoke", post(invoke_plugin_handler))
}

/// Load a plugin manifest from a file
#[allow(dead_code)]
pub fn load_manifest<P: AsRef<StdPath>>(path: P) -> Option<PluginManifest> {
    let path = path.as_ref();
    debug!("Loading manifest from {}", path.display());

    match fs::read_to_string(path) {
        Ok(content) => {
            debug!("Read manifest content: {}", content);
            match toml::from_str(&content) {
                Ok(manifest) => {
                    debug!("Successfully parsed manifest");
                    Some(manifest)
                }
                Err(e) => {
                    error!("Failed to parse manifest: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            error!("Failed to read manifest file: {}", e);
            None
        }
    }
}

/// Save a plugin manifest to a file
#[allow(dead_code)]
pub fn save_manifest<P: AsRef<StdPath>>(manifest: &PluginManifest, path: P) -> std::io::Result<()> {
    let path = path.as_ref();
    debug!("Saving manifest to {}", path.display());

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(manifest).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to serialize manifest: {}", e),
        )
    })?;

    debug!("Writing manifest content: {}", content);
    fs::write(path, content)
}

/// Load a plugin from a manifest file
#[allow(dead_code)]
pub fn load_plugin_from_file<P: AsRef<StdPath>>(path: P) -> Option<Uuid> {
    let path = path.as_ref();
    debug!("Loading plugin from manifest {}", path.display());

    let manifest = load_manifest(path)?;
    debug!("Loaded manifest for plugin {}", manifest.name);

    let mut plugin_manager = PluginManager::with_manifest_dir("plugins");
    match plugin_manager.load_plugin(manifest) {
        Ok(id) => {
            debug!("Successfully loaded plugin with ID {}", id);
            Some(id)
        }
        Err(e) => {
            error!("Failed to load plugin: {}", e);
            None
        }
    }
}

/// Load all plugins from a directory
#[allow(dead_code)]
pub fn load_plugins_from_dir<P: AsRef<StdPath>>(dir: P) -> Vec<Uuid> {
    let dir = dir.as_ref();
    debug!("Loading plugins from directory {}", dir.display());

    let mut loaded_plugins = Vec::new();
    let mut plugin_manager = PluginManager::with_manifest_dir("plugins");

    // Read directory entries
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("Failed to read directory: {}", e);
            return loaded_plugins;
        }
    };

    // Process each entry
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                error!("Failed to read directory entry: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Skip non-manifest files
        if !path.is_file() || path.extension().map_or(true, |ext| ext != "toml") {
            continue;
        }

        debug!("Found manifest file: {}", path.display());

        // Load the manifest
        let manifest = match load_manifest(&path) {
            Some(manifest) => manifest,
            None => continue,
        };

        // Load the plugin
        if let Ok(plugin_id) = plugin_manager.load_plugin(manifest.clone()) {
            debug!(
                "Successfully loaded plugin {} with ID {}",
                manifest.name, plugin_id
            );
            loaded_plugins.push(plugin_id);
        }
    }

    debug!("Loaded {} plugins", loaded_plugins.len());
    loaded_plugins
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn create_test_manifest() -> PluginManifest {
        PluginManifest {
            name: "test_plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "nonexistent".to_string(),
            permissions: vec![],
            driver: None,
            functions: HashMap::new(),
        }
    }

    #[test]
    fn test_save_and_load_manifest() {
        let temp_dir = tempdir().unwrap();
        let manifest_path = temp_dir.path().join("manifest.toml");

        let manifest = create_test_manifest();
        save_manifest(&manifest, &manifest_path).unwrap();

        let loaded = load_manifest(&manifest_path).unwrap();
        assert_eq!(loaded.name, manifest.name);
        assert_eq!(loaded.version, manifest.version);
        assert_eq!(loaded.description, manifest.description);
    }

    #[test]
    fn test_load_plugin_from_file() {
        let temp_dir = tempdir().unwrap();
        let manifest_path = temp_dir.path().join("manifest.toml");

        let manifest = create_test_manifest();
        save_manifest(&manifest, &manifest_path).unwrap();

        assert!(load_plugin_from_file(&manifest_path).is_none());
    }

    #[test]
    fn test_load_plugins_from_dir() {
        let temp_dir = tempdir().unwrap();
        let manifest_path = temp_dir.path().join("test_plugin.toml");

        let manifest = create_test_manifest();
        save_manifest(&manifest, &manifest_path).unwrap();

        let loaded = load_plugins_from_dir(temp_dir.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_plugin_info_from_manifest() {
        let manifest = create_test_manifest();
        let info = PluginInfo::from(&manifest);
        assert_eq!(info.name, manifest.name);
        assert_eq!(info.version, manifest.version);
        assert_eq!(info.description, manifest.description);
    }
}
