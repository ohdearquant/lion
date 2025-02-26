//! Concurrency management for the Lion runtime.
//!
//! This module provides concurrent execution capabilities,
//! including automatic scaling and load balancing.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;

use core::error::{Result, ConcurrencyError};
use core::types::{PluginId, PluginConfig};

use crate::pool::{InstancePool, PoolConfig};

/// Interface for concurrency management.
pub trait ConcurrencyManager: Send + Sync {
    /// Call a function with automatic scaling.
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>>;
    
    /// Configure scaling for a plugin.
    fn configure_scaling(
        &self,
        plugin_id: &PluginId,
        min_instances: usize,
        max_instances: usize,
    ) -> Result<()>;
    
    /// Get the current instance count for a plugin.
    fn get_instance_count(&self, plugin_id: &PluginId) -> Result<usize>;
    
    /// Clean up idle instances.
    fn cleanup_idle(&self) -> Result<usize>;
    
    /// Shutdown and clean up all resources.
    fn shutdown(&self) -> Result<()>;
}

/// Simplified pooled instance for public API.
pub struct PooledInstance {
    /// The plugin ID.
    pub plugin_id: PluginId,
    
    /// The instance ID.
    pub id: usize,
    
    /// The instance age.
    pub age: Duration,
    
    /// The instance idle time.
    pub idle_time: Duration,
}

/// Configuration for the auto-scaler.
#[derive(Clone, Debug)]
pub struct ScalerConfig {
    /// How often to check for scaling.
    pub check_interval: Duration,
    
    /// How often to clean up idle instances.
    pub cleanup_interval: Duration,
    
    /// Scale up when pool utilization exceeds this percentage.
    pub scale_up_threshold: f64,
    
    /// Scale down when pool utilization falls below this percentage.
    pub scale_down_threshold: f64,
    
    /// Maximum number of instances to add per scaling event.
    pub max_scale_up: usize,
    
    /// Maximum number of instances to remove per scaling event.
    pub max_scale_down: usize,
}

impl Default for ScalerConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(10),
            cleanup_interval: Duration::from_secs(30),
            scale_up_threshold: 0.8,  // 80%
            scale_down_threshold: 0.2, // 20%
            max_scale_up: 2,
            max_scale_down: 1,
        }
    }
}

/// Auto-scaling concurrency manager.
pub struct AutoScaler {
    /// The pools for each plugin.
    pools: DashMap<PluginId, Arc<InstancePool>>,
    
    /// Scaler configuration.
    config: ScalerConfig,
    
    /// Whether the scaler is running.
    running: std::sync::atomic::AtomicBool,
}

impl AutoScaler {
    /// Create a new auto-scaler.
    pub fn new(config: ScalerConfig) -> Self {
        Self {
            pools: DashMap::new(),
            config,
            running: std::sync::atomic::AtomicBool::new(true),
        }
    }
    
    /// Add a pool to the scaler.
    pub fn add_pool(&self, plugin_id: PluginId, pool: Arc<InstancePool>) {
        self.pools.insert(plugin_id, pool);
    }
    
    /// Remove a pool from the scaler.
    pub fn remove_pool(&self, plugin_id: &PluginId) -> Result<()> {
        self.pools.remove(plugin_id);
        Ok(())
    }
    
    /// Start the scaler background tasks.
    #[cfg(feature = "async")]
    pub fn start_background_tasks(&self) {
        let scaler = self.clone();
        let cleaner = self.clone();
        
        // Start the scaling task
        tokio::spawn(async move {
            let check_interval = scaler.config.check_interval;
            
            while scaler.running.load(std::sync::atomic::Ordering::SeqCst) {
                scaler.perform_scaling().unwrap_or_else(|e| {
                    log::error!("Error during scaling: {}", e);
                });
                
                tokio::time::sleep(check_interval).await;
            }
        });
        
        // Start the cleanup task
        tokio::spawn(async move {
            let cleanup_interval = cleaner.config.cleanup_interval;
            
            while cleaner.running.load(std::sync::atomic::Ordering::SeqCst) {
                cleaner.cleanup_idle().unwrap_or_else(|e| {
                    log::error!("Error during cleanup: {}", e);
                });
                
                tokio::time::sleep(cleanup_interval).await;
            }
        });
    }
    
    /// Perform scaling checks and adjustments.
    fn perform_scaling(&self) -> Result<()> {
        // ToDo: Implement auto-scaling logic
        Ok(())
    }
}

impl ConcurrencyManager for AutoScaler {
    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        // Get the pool
        let pool = self.pools.get(plugin_id)
            .ok_or(ConcurrencyError::InstanceCreationFailed(
                format!("No pool found for plugin {}", plugin_id)
            ))?;
        
        // Acquire an instance
        let instance = pool.acquire()?;
        
        // Call the function
        instance.call_function(function, params)
    }
    
    fn configure_scaling(
        &self,
        plugin_id: &PluginId,
        min_instances: usize,
        max_instances: usize,
    ) -> Result<()> {
        // Get the pool
        let pool = self.pools.get(plugin_id)
            .ok_or(ConcurrencyError::InstanceCreationFailed(
                format!("No pool found for plugin {}", plugin_id)
            ))?;
        
        // Update the configuration
        let config = PoolConfig {
            min_instances,
            max_instances,
            ..Default::default()
        };
        
        pool.update_config(config)
    }
    
    fn get_instance_count(&self, plugin_id: &PluginId) -> Result<usize> {
        // Get the pool
        let pool = self.pools.get(plugin_id)
            .ok_or(ConcurrencyError::InstanceCreationFailed(
                format!("No pool found for plugin {}", plugin_id)
            ))?;
        
        // Get the count
        Ok(pool.count.load(std::sync::atomic::Ordering::SeqCst))
    }
    
    fn cleanup_idle(&self) -> Result<usize> {
        let mut total_cleaned = 0;
        
        // Clean up each pool
        for pool_entry in self.pools.iter() {
            let pool = pool_entry.value();
            total_cleaned += pool.cleanup_idle()?;
        }
        
        Ok(total_cleaned)
    }
    
    fn shutdown(&self) -> Result<()> {
        // Stop the background tasks
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        
        // Shut down each pool
        for pool_entry in self.pools.iter() {
            let pool = pool_entry.value();
            pool.shutdown()?;
        }
        
        // Clear the pools
        self.pools.clear();
        
        Ok(())
    }
}

impl Clone for AutoScaler {
    fn clone(&self) -> Self {
        Self {
            pools: self.pools.clone(),
            config: self.config.clone(),
            running: std::sync::atomic::AtomicBool::new(
                self.running.load(std::sync::atomic::Ordering::SeqCst)
            ),
        }
    }
}