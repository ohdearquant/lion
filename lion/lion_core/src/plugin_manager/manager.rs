use super::{
    error::PluginError,
    loader::PluginLoader,
    manifest::{PluginManifest, LanguageCapabilities, SecuritySettings},
    registry::PluginMetadata,
    Result,
};
use crate::types::{
    plugin::PluginState,
    traits::{LanguageMessage, LanguageMessageType},
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Resource usage metrics for a plugin
#[derive(Debug, Clone, Default)]
pub struct PluginResourceMetrics {
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// CPU time used in milliseconds
    pub cpu_time_ms: u64,
    /// Number of network requests made
    pub network_requests: usize,
    /// Number of file system operations
    pub fs_operations: usize,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

/// Main plugin management interface that coordinates loading, initialization,
/// and invocation of plugins with support for language network protocol
#[derive(Debug)]
pub struct PluginManager {
    loader: PluginLoader,
    manifest_dir: Option<PathBuf>,
    // Resource monitoring
    resource_metrics: Arc<RwLock<HashMap<Uuid, PluginResourceMetrics>>>,
    // Active language processing plugins
    language_processors: Arc<RwLock<HashMap<Uuid, LanguageCapabilities>>>,
    // Security settings
    security_settings: Arc<RwLock<HashMap<Uuid, SecuritySettings>>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        debug!("Creating new PluginManager with no manifest directory");
        Self {
            loader: PluginLoader::new("data"),
            manifest_dir: None,
            resource_metrics: Arc::new(RwLock::new(HashMap::new())),
            language_processors: Arc::new(RwLock::new(HashMap::new())),
            security_settings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new plugin manager with a manifest directory
    pub fn with_manifest_dir<P: AsRef<Path>>(manifest_dir: P) -> Self {
        let manifest_dir = manifest_dir.as_ref().to_path_buf();
        debug!(
            "Creating new PluginManager with manifest directory: {:?}",
            manifest_dir
        );
        Self {
            loader: PluginLoader::new(manifest_dir.join("data").to_str().unwrap()),
            manifest_dir: Some(manifest_dir),
            resource_metrics: Arc::new(RwLock::new(HashMap::new())),
            language_processors: Arc::new(RwLock::new(HashMap::new())),
            security_settings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the manifest directory
    pub fn manifest_dir(&self) -> Option<&Path> {
        self.manifest_dir.as_deref()
    }

    /// Load a plugin from a manifest file
    pub async fn load_plugin<P: AsRef<Path>>(&self, manifest_path: P) -> Result<Uuid> {
        let manifest_path = manifest_path.as_ref();
        info!("Loading plugin from manifest: {:?}", manifest_path);

        let plugin_id = self.loader.load_from_file(manifest_path).await?;
        
        // Initialize resource metrics
        let mut metrics = self.resource_metrics.write().await;
        metrics.insert(plugin_id, PluginResourceMetrics::default());

        // Store language capabilities if present
        let metadata = self.get_plugin(plugin_id)?;
        if metadata.manifest.language_capabilities.language_processor {
            let mut processors = self.language_processors.write().await;
            processors.insert(plugin_id, metadata.manifest.language_capabilities.clone());
        }

        // Store security settings
        let mut security = self.security_settings.write().await;
        security.insert(plugin_id, metadata.manifest.security.clone());

        Ok(plugin_id)
    }

    /// Load a plugin from a manifest string
    pub async fn load_plugin_from_string(
        &self,
        manifest: String,
        manifest_path: Option<String>,
    ) -> Result<Uuid> {
        info!("Loading plugin from string manifest");

        let manifest: PluginManifest = toml::from_str(&manifest)
            .map_err(|e| PluginError::LoadError(format!("Failed to parse manifest: {}", e)))?;

        let plugin_id = self.loader.load_plugin(manifest.clone(), manifest_path).await?;

        // Initialize resource metrics
        let mut metrics = self.resource_metrics.write().await;
        metrics.insert(plugin_id, PluginResourceMetrics::default());

        // Store language capabilities if present
        if manifest.language_capabilities.language_processor {
            let mut processors = self.language_processors.write().await;
            processors.insert(plugin_id, manifest.language_capabilities.clone());
        }

        // Store security settings
        let mut security = self.security_settings.write().await;
        security.insert(plugin_id, manifest.security.clone());

        Ok(plugin_id)
    }

    /// Get plugin metadata by ID
    pub fn get_plugin(&self, plugin_id: Uuid) -> Result<PluginMetadata> {
        self.loader.registry().get(plugin_id)
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.loader.registry().list()
    }

    /// List all language processing plugins
    pub async fn list_language_processors(&self) -> Vec<(Uuid, LanguageCapabilities)> {
        let processors = self.language_processors.read().await;
        processors.iter().map(|(&id, caps)| (id, caps.clone())).collect()
    }

    /// Check if a plugin can process a specific language message type
    pub async fn can_process_message(&self, plugin_id: Uuid, _message_type: &LanguageMessageType) -> bool {
        let processors = self.language_processors.read().await;
        if let Some(caps) = processors.get(&plugin_id) {
            caps.language_processor && (caps.can_generate || caps.can_modify)
        } else {
            false
        }
    }

    /// Process a language message through a plugin
    pub async fn process_language_message(
        &self,
        plugin_id: Uuid,
        message: LanguageMessage,
    ) -> Result<LanguageMessage> {
        // Check plugin capabilities
        if !self.can_process_message(plugin_id, &message.message_type).await {
            return Err(PluginError::InvokeError(
                "Plugin cannot process this message type".to_string(),
            ));
        }

        // Check security settings
        let security = self.security_settings.read().await;
        let settings = security.get(&plugin_id).ok_or_else(|| {
            PluginError::InvokeError("Plugin security settings not found".to_string())
        })?;

        // Enforce security restrictions
        if !settings.sandboxed {
            warn!("Plugin {} is not running in sandbox mode", plugin_id);
        }

        // Convert message to plugin input format
        let input = serde_json::to_string(&message)
            .map_err(|e| PluginError::InvokeError(format!("Failed to serialize message: {}", e)))?;

        // Process through plugin
        let output = self.invoke_plugin(plugin_id, &input).await?;

        // Parse response
        let response: LanguageMessage = serde_json::from_str(&output)
            .map_err(|e| PluginError::InvokeError(format!("Failed to parse plugin response: {}", e)))?;

        Ok(response)
    }

    /// Invoke a plugin with input
    pub async fn invoke_plugin(&self, plugin_id: Uuid, input: &str) -> Result<String> {
        debug!("Invoking plugin {} with input: {}", plugin_id, input);

        // Get plugin metadata
        let metadata = self.get_plugin(plugin_id)?;

        // Check plugin state
        match metadata.state {
            PluginState::Ready => (),
            PluginState::Uninitialized => {
                return Err(PluginError::InvokeError(
                    "Plugin is not initialized".to_string(),
                ))
            }
            PluginState::Initializing => {
                return Err(PluginError::InvokeError(
                    "Plugin is still initializing".to_string(),
                ))
            }
            PluginState::Running => {
                return Err(PluginError::InvokeError(
                    "Plugin is already running".to_string(),
                ))
            }
            PluginState::Error => {
                return Err(PluginError::InvokeError(
                    "Plugin is in error state".to_string(),
                ))
            }
            PluginState::Disabled => {
                return Err(PluginError::InvokeError("Plugin is disabled".to_string()))
            }
            PluginState::ProcessingLanguage => {
                return Err(PluginError::InvokeError(
                    "Plugin is processing a language message".to_string(),
                ))
            }
        }

        // Check resource limits
        let mut metrics = self.resource_metrics.write().await;
        let plugin_metrics = metrics.entry(plugin_id).or_default();

        let security = self.security_settings.read().await;
        if let Some(settings) = security.get(&plugin_id) {
            if plugin_metrics.memory_usage >= settings.memory_limit_mb * 1024 * 1024 {
                return Err(PluginError::InvokeError("Memory limit exceeded".to_string()));
            }
        }

        // Update metrics
        plugin_metrics.last_activity = Utc::now();
        plugin_metrics.network_requests += 1;

        // TODO: Implement actual WASM invocation with proper sandboxing
        // For now, return a mock response
        Ok(format!(
            "Invoked plugin {} ({}) with input: {}",
            metadata.manifest.name, plugin_id, input
        ))
    }

    /// Remove a plugin
    pub async fn remove_plugin(&self, plugin_id: Uuid) -> Result<()> {
        info!("Removing plugin {}", plugin_id);

        // Get plugin metadata for logging
        if let Ok(metadata) = self.get_plugin(plugin_id) {
            debug!("Removing plugin: {}", metadata.manifest.name);
        }

        // Clean up resources
        let mut metrics = self.resource_metrics.write().await;
        metrics.remove(&plugin_id);

        let mut processors = self.language_processors.write().await;
        processors.remove(&plugin_id);

        let mut security = self.security_settings.write().await;
        security.remove(&plugin_id);

        self.loader.registry().remove(plugin_id)
    }

    /// Get resource metrics for a plugin
    pub async fn get_resource_metrics(&self, plugin_id: Uuid) -> Option<PluginResourceMetrics> {
        let metrics = self.resource_metrics.read().await;
        metrics.get(&plugin_id).cloned()
    }

    /// Check if a plugin has exceeded its resource limits
    pub async fn check_resource_limits(&self, plugin_id: Uuid) -> Result<bool> {
        let metrics = self.resource_metrics.read().await;
        let security = self.security_settings.read().await;

        let plugin_metrics = metrics.get(&plugin_id).ok_or_else(|| {
            PluginError::NotFound(plugin_id)
        })?;

        let settings = security.get(&plugin_id).ok_or_else(|| {
            PluginError::NotFound(plugin_id)
        })?;

        Ok(plugin_metrics.memory_usage >= settings.memory_limit_mb * 1024 * 1024
            || plugin_metrics.cpu_time_ms >= settings.time_limit_secs as u64 * 1000)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_plugin_manager() {
        let temp_dir = tempdir().unwrap();
        let manager = PluginManager::with_manifest_dir(temp_dir.path());

        // Create test manifest with language capabilities
        let mut manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        manifest.language_capabilities.language_processor = true;
        manifest.language_capabilities.supported_models.insert("gpt-4".to_string());
        manifest.language_capabilities.can_generate = true;

        manifest.security.sandboxed = true;
        manifest.security.memory_limit_mb = 512;
        manifest.security.time_limit_secs = 30;

        // Load plugin
        let plugin_id = manager
            .load_plugin_from_string(toml::to_string(&manifest).unwrap(), None)
            .await
            .unwrap();

        // Get plugin
        let metadata = manager.get_plugin(plugin_id).unwrap();
        assert_eq!(metadata.manifest.name, "test-plugin");
        assert_eq!(metadata.manifest.version, "1.0.0");
        assert_eq!(metadata.state, PluginState::Ready, "Plugin state should be Ready after initialization");

        // Check language capabilities
        let processors = manager.list_language_processors().await;
        assert_eq!(processors.len(), 1);
        assert_eq!(processors[0].0, plugin_id);
        assert!(processors[0].1.language_processor);
        assert!(processors[0].1.can_generate);

        // Check resource metrics
        let metrics = manager.get_resource_metrics(plugin_id).await.unwrap();
        assert_eq!(metrics.memory_usage, 0);
        assert_eq!(metrics.network_requests, 0);

        // Test message processing
        let message = LanguageMessage {
            id: Uuid::new_v4(),
            content: "test message".to_string(),
            sender_id: Uuid::new_v4(),
            recipient_ids: vec![Uuid::new_v4()].into_iter().collect(),
            message_type: LanguageMessageType::Text,
            metadata: serde_json::json!({}),
            timestamp: Utc::now(),
        };

        let can_process = manager.can_process_message(plugin_id, &message.message_type).await;
        assert!(can_process);

        // Remove plugin
        manager.remove_plugin(plugin_id).await.unwrap();
        assert!(manager.get_plugin(plugin_id).is_err());
    }

    #[tokio::test]
    async fn test_invalid_plugin() {
        let manager = PluginManager::new();

        // Create invalid manifest
        let manifest = PluginManifest::new(
            "".to_string(),
            "1.0.0".to_string(),
            "".to_string(),
        );

        // Try to load plugin
        let result = manager
            .load_plugin_from_string(toml::to_string(&manifest).unwrap(), None)
            .await;
        assert!(matches!(result, Err(PluginError::InvalidManifest(_))));
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let manager = PluginManager::new();

        // Create test manifest with strict limits
        let mut manifest = PluginManifest::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            "A test plugin".to_string(),
        );

        manifest.security.memory_limit_mb = 1;
        manifest.security.time_limit_secs = 1;

        // Load plugin
        let plugin_id = manager
            .load_plugin_from_string(toml::to_string(&manifest).unwrap(), None)
            .await
            .unwrap();

        // Simulate resource usage
        {
            let mut metrics = manager.resource_metrics.write().await;
            let plugin_metrics = metrics.get_mut(&plugin_id).unwrap();
            plugin_metrics.memory_usage = 2 * 1024 * 1024; // 2MB
            plugin_metrics.cpu_time_ms = 2000; // 2 seconds
        }

        // Check limits
        let exceeded = manager.check_resource_limits(plugin_id).await.unwrap();
        assert!(exceeded);
    }
}
