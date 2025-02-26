//! Lion Runtime - Core runtime for the Lion plugin system
//!
//! This crate integrates all components of the Lion plugin system
//! into a cohesive runtime environment.

mod plugin;
mod config;

pub use plugin::{PluginManager, PluginRegistry};
pub use config::{RuntimeConfig, load_config, save_config};

use std::sync::Arc;
use std::path::Path;

use dashmap::DashMap;
use core::error::Result;
use core::types::{PluginId, PluginState, PluginType, PluginConfig, PluginMetadata};
use capabilities::CapabilityManager;
use policy::PolicyManager;
use isolation::{WasmIsolationFactory, WasmIsolationBackend, ModuleStore, MemoryModuleStore};
use concurrency::{InstanceManager, PoolConfig};
use workflow::{WorkflowManager, WorkflowStorage, MemoryWorkflowStorage};

#[cfg(feature = "observability")]
use observability::{ObservabilityHandle, TracingConfig, MetricsConfig};

/// Core runtime for the Lion plugin system.
pub struct Runtime {
    /// Plugin manager.
    plugin_manager: Arc<PluginManager>,
    
    /// Capability manager.
    capability_manager: Arc<CapabilityManager>,
    
    /// Policy manager.
    policy_manager: Arc<PolicyManager>,
    
    /// Workflow manager.
    workflow_manager: Arc<WorkflowManager>,
    
    /// Runtime configuration.
    config: RuntimeConfig,
    
    /// Observability handle.
    #[cfg(feature = "observability")]
    observability_handle: Option<ObservabilityHandle>,
}

impl Runtime {
    /// Create a new Lion runtime.
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        // Initialize observability if enabled
        #[cfg(feature = "observability")]
        let observability_handle = if config.enable_observability {
            let tracing_config = TracingConfig {
                service_name: "lion".to_string(),
                log_level: config.log_level.clone(),
                enable_file_logging: config.enable_file_logging,
                log_directory: config.log_directory.clone(),
                enable_console_logging: config.enable_console_logging,
                enable_json_format: config.enable_json_format,
                enable_jaeger: config.enable_jaeger,
                jaeger_endpoint: config.jaeger_endpoint.clone(),
            };
            
            let metrics_config = MetricsConfig {
                service_name: "lion".to_string(),
                enable_prometheus: config.enable_prometheus,
                prometheus_addr: config.prometheus_addr,
            };
            
            Some(observability::init(Some(tracing_config), Some(metrics_config))?)
        } else {
            None
        };
        
        // Create the module store
        let module_store: Arc<dyn ModuleStore> = Arc::new(MemoryModuleStore::new());
        
        // Create isolation factory
        let isolation_factory = WasmIsolationFactory::new(config.max_memory_bytes)
            .with_store(module_store);
        
        // Create capability manager
        let capability_manager = Arc::new(CapabilityManager::default());
        
        // Create policy manager
        let policy_manager = Arc::new(PolicyManager::default(capability_manager.clone()));
        
        // Create the workflow storage
        let workflow_storage: Arc<dyn WorkflowStorage> = Arc::new(MemoryWorkflowStorage::new());
        
        // Create the plugin manager
        let plugin_manager = PluginManager::new(
            isolation_factory,
            capability_manager.clone(),
            policy_manager.clone(),
            config.clone(),
        )?;
        
        // Create instance manager for concurrency
        let default_pool_config = PoolConfig {
            min_instances: config.default_min_instances,
            max_instances: config.default_max_instances,
            wait_timeout: std::time::Duration::from_millis(config.default_wait_timeout_ms),
            idle_timeout: std::time::Duration::from_secs(config.default_idle_timeout_sec),
        };
        
        let instance_manager = Arc::new(concurrency::InstanceManager::new(
            plugin_manager.get_backend(),
            default_pool_config,
        ));
        
        // Create the workflow engine config
        let workflow_config = workflow::WorkflowConfig {
            default_max_parallel_nodes: config.default_max_parallel_nodes,
            default_timeout_ms: config.default_workflow_timeout_ms,
            default_continue_on_failure: config.default_continue_on_failure,
            default_use_checkpoints: config.default_use_checkpoints,
            default_checkpoint_interval_ms: config.default_checkpoint_interval_ms,
            max_active_executions: config.max_active_executions,
        };
        
        // Create workflow manager
        let workflow_manager = Arc::new(WorkflowManager::new(
            instance_manager,
            workflow_storage,
            workflow_config,
        ));
        
        Ok(Self {
            plugin_manager: Arc::new(plugin_manager),
            capability_manager,
            policy_manager,
            workflow_manager,
            config,
            #[cfg(feature = "observability")]
            observability_handle,
        })
    }
    
    /// Get the plugin manager.
    pub fn plugin_manager(&self) -> Arc<PluginManager> {
        self.plugin_manager.clone()
    }
    
    /// Get the capability manager.
    pub fn capability_manager(&self) -> Arc<CapabilityManager> {
        self.capability_manager.clone()
    }
    
    /// Get the policy manager.
    pub fn policy_manager(&self) -> Arc<PolicyManager> {
        self.policy_manager.clone()
    }
    
    /// Get the workflow manager.
    pub fn workflow_manager(&self) -> Arc<WorkflowManager> {
        self.workflow_manager.clone()
    }
    
    /// Get the runtime configuration.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
    
    /// Load a plugin from a file.
    pub fn load_plugin_from_file(
        &self,
        path: &Path,
        name: &str,
        version: &str,
        description: &str,
    ) -> Result<PluginId> {
        // Read the file
        let code = std::fs::read(path)?;
        
        // Load the plugin
        self.plugin_manager.load_plugin(
            name,
            version,
            description,
            PluginType::Wasm,
            code,
            PluginConfig::default(),
        )
    }
    
    /// Shutdown the runtime.
    pub fn shutdown(&self) -> Result<()> {
        // Shutdown plugin manager
        self.plugin_manager.shutdown()?;
        
        // Shutdown observability
        #[cfg(feature = "observability")]
        if let Some(handle) = &self.observability_handle {
            handle.shutdown()?;
        }
        
        Ok(())
    }
}