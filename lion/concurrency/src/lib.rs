//! Lion Concurrency - Thread-safe instance pooling
//!
//! This crate provides concurrency primitives for the Lion runtime,
//! including instance pooling for parallel function execution.

mod pool;
mod manager;

pub use pool::{InstancePool, PoolConfig};
pub use manager::{ConcurrencyManager, PooledInstance};

use std::sync::Arc;
use dashmap::DashMap;

use core::error::{Result, ConcurrencyError};
use core::traits::IsolationBackend;
use core::types::{PluginId, PluginConfig};

/// Core instance manager that enables parallel execution.
pub struct InstanceManager {
    /// The isolation backend.
    backend: Arc<dyn IsolationBackend>,
    
    /// Instance pools for each plugin.
    pools: DashMap<PluginId, Arc<InstancePool>>,
    
    /// Default pool configuration.
    default_config: PoolConfig,
}

impl InstanceManager {
    /// Create a new instance manager.
    pub fn new(backend: Arc<dyn IsolationBackend>, default_config: PoolConfig) -> Self {
        Self {
            backend,
            pools: DashMap::new(),
            default_config,
        }
    }
    
    /// Create and configure an instance pool for a plugin.
    pub fn configure_pool(
        &self,
        plugin_id: &PluginId,
        config: PoolConfig,
    ) -> Result<Arc<InstancePool>> {
        // Check if the pool already exists
        if let Some(pool) = self.pools.get(plugin_id) {
            // Pool exists, update configuration
            pool.update_config(config)?;
            return Ok(pool.clone());
        }
        
        // Create a new pool
        let pool = Arc::new(InstancePool::new(
            plugin_id.clone(),
            self.backend.clone(),
            config,
        ));
        
        // Store the pool
        self.pools.insert(plugin_id.clone(), pool.clone());
        
        Ok(pool)
    }
    
    /// Get or create an instance pool for a plugin.
    pub fn get_pool(&self, plugin_id: &PluginId) -> Result<Arc<InstancePool>> {
        // Check if the pool exists
        if let Some(pool) = self.pools.get(plugin_id) {
            return Ok(pool.clone());
        }
        
        // Create a new pool with default configuration
        self.configure_pool(plugin_id, self.default_config.clone())
    }
    
    /// Call a function with an instance from the pool.
    pub fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        // Get the pool
        let pool = self.get_pool(plugin_id)?;
        
        // Acquire an instance
        let instance = pool.acquire()?;
        
        // Call the function
        instance.call_function(function, params)
    }
    
    /// Create a new plugin instance pool.
    pub fn create_plugin(
        &self,
        plugin_id: PluginId,
        code: Vec<u8>,
        config: PluginConfig,
        pool_config: Option<PoolConfig>,
    ) -> Result<()> {
        // Load the plugin in the backend
        self.backend.load_plugin(plugin_id.clone(), code, config.clone())?;
        
        // Configure the pool
        let config = pool_config.unwrap_or_else(|| self.default_config.clone());
        let pool = self.configure_pool(&plugin_id, config)?;
        
        // Pre-warm the pool by creating minimum instances
        pool.pre_warm()?;
        
        Ok(())
    }
    
    /// Remove a plugin and its instance pool.
    pub fn remove_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        // Remove the pool
        if let Some((_, pool)) = self.pools.remove(plugin_id) {
            // Shut down the pool
            pool.shutdown()?;
        }
        
        // Unload the plugin from the backend
        self.backend.unload_plugin(plugin_id)?;
        
        Ok(())
    }
    
    /// Shut down all pools.
    pub fn shutdown(&self) -> Result<()> {
        // Get all plugin IDs
        let plugin_ids: Vec<PluginId> = self.pools.iter()
            .map(|entry| entry.key().clone())
            .collect();
        
        // Remove each plugin
        for plugin_id in plugin_ids {
            self.remove_plugin(&plugin_id)?;
        }
        
        Ok(())
    }
}