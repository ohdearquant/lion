//! System initialization and management for the CLI.

use crate::error::CliError;
use lion_capability_manager::{CapabilityManagerImpl, CapabilityManagerImplConfig, DefaultCapabilityPolicy};
use lion_core::capability::{CapabilityManager, CoreCapability};
use lion_core::message::MessageBus;
use lion_core::plugin::{Plugin, PluginId, PluginManager, PluginManifest, PluginSource, PluginState};
use lion_core::resource::{ResourceLimits, ResourceMonitor, ResourceUsage};
use lion_message_bus::InMemoryMessageBus;
use lion_plugin_manager::LionPluginManager;
use lion_wasm_runtime::{WasmRuntime, WasmRuntimeConfig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// Configuration for the Lion system
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SystemConfig {
    /// Default memory limit for plugins in bytes
    #[serde(default = "default_memory_limit")]
    pub default_memory_limit: usize,
    
    /// Default execution time limit for plugins in seconds
    #[serde(default = "default_execution_time_limit")]
    pub default_execution_time_limit: u64,
    
    /// Default allowed file system paths
    #[serde(default)]
    pub allowed_fs_paths: Vec<String>,
    
    /// Default allowed network hosts
    #[serde(default)]
    pub allowed_network_hosts: Vec<String>,
    
    /// Whether to allow all capabilities by default
    #[serde(default)]
    pub allow_all_capabilities: bool,
}

fn default_memory_limit() -> usize {
    100 * 1024 * 1024 // 100 MB
}

fn default_execution_time_limit() -> u64 {
    5 // 5 seconds
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            default_memory_limit: default_memory_limit(),
            default_execution_time_limit: default_execution_time_limit(),
            allowed_fs_paths: vec!["/tmp".to_string()],
            allowed_network_hosts: Vec::new(),
            allow_all_capabilities: false,
        }
    }
}

/// The core Lion system
pub struct LionSystem {
    /// The plugin manager
    plugin_manager: Arc<dyn PluginManager>,
    
    /// The capability manager
    capability_manager: Arc<dyn CapabilityManager>,
    
    /// The message bus
    message_bus: Arc<dyn MessageBus>,
    
    /// The resource monitor
    resource_monitor: Arc<dyn ResourceMonitor>,
    
    /// The WebAssembly runtime
    wasm_runtime: Arc<WasmRuntime>,
    
    /// The system configuration
    config: SystemConfig,
}

impl LionSystem {
    /// Initialize the Lion system
    pub fn initialize(config_path: Option<PathBuf>) -> Result<Self, CliError> {
        // Load configuration
        let config = if let Some(path) = config_path {
            Self::load_config(&path)?
        } else {
            SystemConfig::default()
        };
        
        // Initialize components
        let capability_manager = Self::create_capability_manager(&config)?;
        let message_bus = Self::create_message_bus()?;
        let resource_monitor = Self::create_resource_monitor()?;
        let wasm_runtime = Self::create_wasm_runtime(
            capability_manager.clone(),
            message_bus.clone(),
            &config,
        )?;
        let plugin_manager = Self::create_plugin_manager(
            capability_manager.clone(),
            message_bus.clone(),
            resource_monitor.clone(),
            wasm_runtime.clone(),
        )?;
        
        Ok(Self {
            plugin_manager,
            capability_manager,
            message_bus,
            resource_monitor,
            wasm_runtime,
            config,
        })
    }
    
    /// Load configuration from a file
    fn load_config(path: &Path) -> Result<SystemConfig, CliError> {
        let config_str = std::fs::read_to_string(path)?;
        let config: SystemConfig = toml::from_str(&config_str)?;
        Ok(config)
    }
    
    /// Create a capability manager
    fn create_capability_manager(config: &SystemConfig) -> Result<Arc<dyn CapabilityManager>, CliError> {
        // Create a policy
        let mut policy = DefaultCapabilityPolicy::new(config.allow_all_capabilities);
        
        // Add allowed file system paths
        for path in &config.allowed_fs_paths {
            policy.add_allowed_fs_path(PathBuf::from(path));
        }
        
        // Add allowed network hosts
        for host in &config.allowed_network_hosts {
            policy.add_allowed_network_host(host.clone());
        }
        
        // Create the capability manager
        let capability_config = CapabilityManagerImplConfig {
            policy: Arc::new(policy),
        };
        let capability_manager = Arc::new(CapabilityManagerImpl::new(capability_config));
        
        Ok(capability_manager)
    }
    
    /// Create a message bus
    fn create_message_bus() -> Result<Arc<dyn MessageBus>, CliError> {
        let message_bus = Arc::new(InMemoryMessageBus::new());
        Ok(message_bus)
    }
    
    /// Create a resource monitor
    fn create_resource_monitor() -> Result<Arc<dyn ResourceMonitor>, CliError> {
        let resource_monitor = Arc::new(lion_plugin_manager::ResourceMonitorImpl::new());
        Ok(resource_monitor)
    }
    
    /// Create a WebAssembly runtime
    fn create_wasm_runtime(
        capability_manager: Arc<dyn CapabilityManager>,
        message_bus: Arc<dyn MessageBus>,
        config: &SystemConfig,
    ) -> Result<Arc<WasmRuntime>, CliError> {
        let wasm_config = WasmRuntimeConfig {
            instance_config: lion_wasm_runtime::WasmInstanceConfig {
                memory_limit: config.default_memory_limit,
                execution_time_limit: std::time::Duration::from_secs(config.default_execution_time_limit),
                fuel_limit: Some(10_000_000), // 10 million instructions
            },
            enable_fuel_metering: true,
            default_fuel_limit: Some(10_000_000),
        };
        
        let wasm_runtime = WasmRuntime::new(
            capability_manager,
            message_bus,
            wasm_config,
        ).map_err(|e| CliError::SystemInitialization(format!("Failed to create Wasm runtime: {}", e)))?;
        
        Ok(Arc::new(wasm_runtime))
    }
    
    /// Create a plugin manager
    fn create_plugin_manager(
        capability_manager: Arc<dyn CapabilityManager>,
        message_bus: Arc<dyn MessageBus>,
        resource_monitor: Arc<dyn ResourceMonitor>,
        wasm_runtime: Arc<WasmRuntime>,
    ) -> Result<Arc<dyn PluginManager>, CliError> {
        let plugin_manager = Arc::new(LionPluginManager::new(
            capability_manager,
            message_bus,
            resource_monitor,
            wasm_runtime,
        ));
        
        Ok(plugin_manager)
    }
    
    /// Load a plugin from a manifest file
    pub fn load_plugin(&self, manifest_path: &Path) -> Result<PluginId, CliError> {
        // Read the manifest file
        let manifest_str = std::fs::read_to_string(manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&manifest_str)?;
        
        // Load the plugin
        let plugin_id = self.plugin_manager.load_plugin(&manifest)?;
        
        Ok(plugin_id)
    }
    
    /// Load a plugin from a WebAssembly file
    pub fn load_wasm_plugin(
        &self,
        wasm_path: &Path,
        name: &str,
        capabilities: Option<&str>,
    ) -> Result<PluginId, CliError> {
        // Read the WebAssembly file
        let wasm_bytes = std::fs::read(wasm_path)?;
        
        // Parse capabilities
        let requested_capabilities = if let Some(caps) = capabilities {
            Self::parse_capabilities(caps)?
        } else {
            vec![CoreCapability::InterPluginComm]
        };
        
        // Create a manifest
        let manifest = PluginManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: None,
            author: None,
            source: PluginSource::InMemory(wasm_bytes),
            requested_capabilities,
        };
        
        // Load the plugin
        let plugin_id = self.plugin_manager.load_plugin(&manifest)?;
        
        Ok(plugin_id)
    }
    
    /// Parse a comma-separated list of capabilities
    fn parse_capabilities(caps_str: &str) -> Result<Vec<CoreCapability>, CliError> {
        let mut capabilities = Vec::new();
        
        for cap in caps_str.split(',') {
            let cap = cap.trim();
            if cap.is_empty() {
                continue;
            }
            
            // Parse the capability
            if cap == "fs:read" {
                capabilities.push(CoreCapability::FileSystemRead { path: None });
            } else if cap.starts_with("fs:read:") {
                let path = cap.strip_prefix("fs:read:").unwrap().to_string();
                capabilities.push(CoreCapability::FileSystemRead { path: Some(path) });
            } else if cap == "fs:write" {
                capabilities.push(CoreCapability::FileSystemWrite { path: None });
            } else if cap.starts_with("fs:write:") {
                let path = cap.strip_prefix("fs:write:").unwrap().to_string();
                capabilities.push(CoreCapability::FileSystemWrite { path: Some(path) });
            } else if cap == "net" {
                capabilities.push(CoreCapability::NetworkClient { hosts: None });
            } else if cap.starts_with("net:") {
                let hosts = cap.strip_prefix("net:").unwrap()
                    .split('|')
                    .map(|s| s.to_string())
                    .collect();
                capabilities.push(CoreCapability::NetworkClient { hosts: Some(hosts) });
            } else if cap == "ipc" {
                capabilities.push(CoreCapability::InterPluginComm);
            } else {
                return Err(CliError::InvalidArguments(format!("Unknown capability: {}", cap)));
            }
        }
        
        Ok(capabilities)
    }
    
    /// Get a plugin by ID
    pub fn get_plugin(&self, id_str: &str) -> Result<Arc<dyn Plugin>, CliError> {
        // Parse the ID
        let id = PluginId(Uuid::parse_str(id_str)
            .map_err(|_| CliError::InvalidPluginId(id_str.to_string()))?);
        
        // Get the plugin
        self.plugin_manager.get_plugin(id)
            .ok_or_else(|| CliError::InvalidPluginId(id_str.to_string()))
    }
    
    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<(PluginId, String, PluginState)> {
        let mut result = Vec::new();
        
        for plugin_id in self.plugin_manager.list_plugins() {
            if let Some(plugin) = self.plugin_manager.get_plugin(plugin_id) {
                result.push((plugin_id, plugin.name().to_string(), plugin.state()));
            }
        }
        
        result
    }
    
    /// Get plugin manifest
    pub fn get_manifest(&self, id_str: &str) -> Result<PluginManifest, CliError> {
        // Parse the ID
        let id = PluginId(Uuid::parse_str(id_str)
            .map_err(|_| CliError::InvalidPluginId(id_str.to_string()))?);
        
        // Get the manifest
        self.plugin_manager.get_manifest(id)
            .ok_or_else(|| CliError::InvalidPluginId(id_str.to_string()))
    }
    
    /// Unload a plugin
    pub fn unload_plugin(&self, id_str: &str) -> Result<(), CliError> {
        // Parse the ID
        let id = PluginId(Uuid::parse_str(id_str)
            .map_err(|_| CliError::InvalidPluginId(id_str.to_string()))?);
        
        // Unload the plugin
        self.plugin_manager.unload_plugin(id)?;
        
        Ok(())
    }
    
    /// Send a message to a plugin
    pub fn send_message(&self, id_str: &str, message_json: &str) -> Result<Option<serde_json::Value>, CliError> {
        // Parse the ID
        let id = PluginId(Uuid::parse_str(id_str)
            .map_err(|_| CliError::InvalidPluginId(id_str.to_string()))?);
        
        // Parse the message
        let message: serde_json::Value = serde_json::from_str(message_json)?;
        
        // Get the plugin
        let mut plugin = self.get_plugin(id_str)?;
        
        // Initialize the plugin if needed
        if plugin.state() == PluginState::Created {
            plugin.initialize()?;
        }
        
        // Send the message
        let result = plugin.handle_message(message)?;
        
        Ok(result)
    }
    
    /// Get the capabilities granted to a plugin
    pub fn get_capabilities(&self, id_str: &str) -> Result<Vec<CoreCapability>, CliError> {
        // Parse the ID
        let id = PluginId(Uuid::parse_str(id_str)
            .map_err(|_| CliError::InvalidPluginId(id_str.to_string()))?);
        
        // Get the capabilities
        let caps = if let Some(cm) = self.capability_manager.as_any().downcast_ref::<CapabilityManagerImpl>() {
            cm.get_granted_capabilities(id)
        } else {
            Vec::new()
        };
        
        Ok(caps)
    }
    
    /// Get resource usage for a plugin
    pub fn get_resource_usage(&self, id_str: &str) -> Result<ResourceUsage, CliError> {
        // Parse the ID
        let id = PluginId(Uuid::parse_str(id_str)
            .map_err(|_| CliError::InvalidPluginId(id_str.to_string()))?);
        
        // Get the resource usage
        self.resource_monitor.get_usage(id).map_err(Into::into)
    }
    
    /// Create a plugin chain
    pub fn create_chain(&self, ids_str: &str) -> Result<Vec<PluginId>, CliError> {
        let mut chain = Vec::new();
        
        // Parse the comma-separated list of IDs
        for id_str in ids_str.split(',') {
            let id_str = id_str.trim();
            if id_str.is_empty() {
                continue;
            }
            
            // Parse the ID
            let id = PluginId(Uuid::parse_str(id_str)
                .map_err(|_| CliError::InvalidPluginId(id_str.to_string()))?);
            
            // Check if the plugin exists
            if self.plugin_manager.get_plugin(id).is_none() {
                return Err(CliError::InvalidPluginId(id_str.to_string()));
            }
            
            chain.push(id);
        }
        
        if chain.is_empty() {
            return Err(CliError::InvalidArguments("Chain must contain at least one plugin".to_string()));
        }
        
        Ok(chain)
    }
    
    /// Execute a plugin chain
    pub fn execute_chain(&self, chain: &[PluginId], input: Option<&str>) -> Result<Option<serde_json::Value>, CliError> {
        if chain.is_empty() {
            return Err(CliError::Chain("Chain is empty".to_string()));
        }
        
        // Parse input if provided
        let mut message = if let Some(input_str) = input {
            serde_json::from_str(input_str)?
        } else {
            serde_json::json!({})
        };
        
        // Process the chain
        for (i, &plugin_id) in chain.iter().enumerate() {
            // Get the plugin
            let plugin = self.plugin_manager.get_plugin(plugin_id)
                .ok_or_else(|| CliError::Chain(format!("Plugin not found: {}", plugin_id.0)))?;
            
            // Handle the message
            let mut plugin_clone = plugin.clone();
            
            // Initialize if needed
            if plugin_clone.state() == PluginState::Created {
                plugin_clone.initialize()?;
            }
            
            // Process the message
            let result = plugin_clone.handle_message(message)?;
            
            // Use the result as the input for the next plugin
            if let Some(result_value) = result {
                message = result_value;
            } else if i < chain.len() - 1 {
                // If a plugin returns None but it's not the last one, that's an error
                return Err(CliError::Chain(format!("Plugin {} returned no result", plugin_id.0)));
            }
        }
        
        Ok(Some(message))
    }
    
    /// Get message bus for inter-plugin communication
    pub fn message_bus(&self) -> Arc<dyn MessageBus> {
        self.message_bus.clone()
    }
    
    /// Get capability manager
    pub fn capability_manager(&self) -> Arc<dyn CapabilityManager> {
        self.capability_manager.clone()
    }
    
    /// Get resource monitor
    pub fn resource_monitor(&self) -> Arc<dyn ResourceMonitor> {
        self.resource_monitor.clone()
    }
    
    /// Get plugin manager
    pub fn plugin_manager(&self) -> Arc<dyn PluginManager> {
        self.plugin_manager.clone()
    }
    
    /// Get WebAssembly runtime
    pub fn wasm_runtime(&self) -> Arc<WasmRuntime> {
        self.wasm_runtime.clone()
    }
}