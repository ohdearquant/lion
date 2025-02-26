//! Instance pooling for parallel execution.
//!
//! This module provides a thread-safe instance pool that can
//! maintain multiple instances of a plugin for parallel execution.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crossbeam_queue::ArrayQueue;
use parking_lot::{Mutex, Condvar};

use core::error::{Result, ConcurrencyError};
use core::traits::IsolationBackend;
use core::types::{PluginId, PluginConfig};

/// Configuration for an instance pool.
#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Minimum number of instances to keep ready.
    pub min_instances: usize,
    
    /// Maximum number of instances allowed.
    pub max_instances: usize,
    
    /// How long to wait for an instance before creating a new one.
    pub wait_timeout: Duration,
    
    /// How long an idle instance can live before being removed.
    pub idle_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_instances: 1,
            max_instances: 10,
            wait_timeout: Duration::from_millis(100),
            idle_timeout: Duration::from_secs(60),
        }
    }
}

/// A pooled plugin instance.
pub struct PooledInstance {
    /// The instance ID.
    id: usize,
    
    /// The plugin ID.
    plugin_id: PluginId,
    
    /// The isolation backend.
    backend: Arc<dyn IsolationBackend>,
    
    /// When this instance was created.
    created_at: Instant,
    
    /// When this instance was last used.
    last_used: Mutex<Instant>,
    
    /// Whether this instance is active.
    active: bool,
    
    /// Reference to the pool for return.
    pool: Option<Arc<InstancePool>>,
}

impl PooledInstance {
    /// Create a new pooled instance.
    fn new(
        id: usize,
        plugin_id: PluginId,
        backend: Arc<dyn IsolationBackend>,
        pool: Arc<InstancePool>,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            plugin_id,
            backend,
            created_at: now,
            last_used: Mutex::new(now),
            active: true,
            pool: Some(pool),
        }
    }
    
    /// Call a function on this instance.
    pub fn call_function(&self, function: &str, params: &[u8]) -> Result<Vec<u8>> {
        // Update the last used time
        *self.last_used.lock() = Instant::now();
        
        // Call the function
        self.backend.call_function(&self.plugin_id, function, params)
    }
    
    /// Get the age of this instance.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
    
    /// Get the idle time of this instance.
    pub fn idle_time(&self) -> Duration {
        self.last_used.lock().elapsed()
    }
}

impl Drop for PooledInstance {
    fn drop(&mut self) {
        // Return to the pool if active
        if self.active {
            if let Some(pool) = self.pool.take() {
                pool.return_instance(self.id);
            }
        }
    }
}

/// A pool of plugin instances for concurrent execution.
pub struct InstancePool {
    /// The plugin ID.
    plugin_id: PluginId,
    
    /// The isolation backend.
    backend: Arc<dyn IsolationBackend>,
    
    /// The pool configuration.
    config: Mutex<PoolConfig>,
    
    /// Available instances.
    available: Arc<ArrayQueue<usize>>,
    
    /// All instances (active and available).
    instances: Mutex<Vec<Option<Arc<PooledInstance>>>>,
    
    /// Total number of instances.
    count: AtomicUsize,
    
    /// Next instance ID.
    next_id: AtomicUsize,
    
    /// Condition variable for waiting for instances.
    condvar: Condvar,
    
    /// Whether the pool is shutting down.
    shutting_down: AtomicUsize,
}

impl InstancePool {
    /// Create a new instance pool.
    pub fn new(
        plugin_id: PluginId,
        backend: Arc<dyn IsolationBackend>,
        config: PoolConfig,
    ) -> Self {
        let available = Arc::new(ArrayQueue::new(config.max_instances));
        
        Self {
            plugin_id,
            backend,
            config: Mutex::new(config),
            available,
            instances: Mutex::new(Vec::new()),
            count: AtomicUsize::new(0),
            next_id: AtomicUsize::new(0),
            condvar: Condvar::new(),
            shutting_down: AtomicUsize::new(0),
        }
    }
    
    /// Update the pool configuration.
    pub fn update_config(&self, config: PoolConfig) -> Result<()> {
        let mut current_config = self.config.lock();
        
        // Validate the config
        if config.min_instances > config.max_instances {
            return Err(ConcurrencyError::PoolLimitReached(
                "Minimum instances cannot be greater than maximum instances".to_string()
            ).into());
        }
        
        // Update the config
        *current_config = config;
        
        Ok(())
    }
    
    /// Pre-warm the pool by creating minimum instances.
    pub fn pre_warm(&self) -> Result<()> {
        let config = self.config.lock();
        let min_instances = config.min_instances;
        
        // Create minimum instances
        for _ in 0..min_instances {
            self.create_instance()?;
        }
        
        Ok(())
    }
    
    /// Create a new instance.
    fn create_instance(&self) -> Result<usize> {
        // Check if we're at the maximum
        let config = self.config.lock();
        let count = self.count.load(Ordering::SeqCst);
        
        if count >= config.max_instances {
            return Err(ConcurrencyError::PoolLimitReached(
                format!("Maximum instances ({}) reached", config.max_instances)
            ).into());
        }
        
        // Get the next instance ID
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        
        // Create a reference to the pool
        let pool = Arc::new(self.clone());
        
        // Create the instance
        let instance = Arc::new(PooledInstance::new(
            id,
            self.plugin_id.clone(),
            self.backend.clone(),
            pool,
        ));
        
        // Store the instance
        let mut instances = self.instances.lock();
        if id >= instances.len() {
            instances.resize_with(id + 1, || None);
        }
        instances[id] = Some(instance);
        
        // Increment the count
        self.count.fetch_add(1, Ordering::SeqCst);
        
        // Add to available queue
        self.available.push(id).map_err(|_| {
            ConcurrencyError::PoolLimitReached("Failed to add instance to queue".to_string())
        })?;
        
        // Notify waiters
        self.condvar.notify_one();
        
        Ok(id)
    }
    
    /// Acquire an instance from the pool.
    pub fn acquire(&self) -> Result<Arc<PooledInstance>> {
        // Check if we're shutting down
        if self.shutting_down.load(Ordering::SeqCst) > 0 {
            return Err(ConcurrencyError::PoolLimitReached("Pool is shutting down".to_string()).into());
        }
        
        // Try to get an available instance
        if let Some(id) = self.available.pop() {
            // Get the instance
            let instances = self.instances.lock();
            if let Some(Some(instance)) = instances.get(id) {
                return Ok(instance.clone());
            }
        }
        
        // No available instance, create a new one if we can
        let config = self.config.lock();
        let count = self.count.load(Ordering::SeqCst);
        
        if count < config.max_instances {
            // Create a new instance
            let id = self.create_instance()?;
            
            // Get the instance
            let instances = self.instances.lock();
            if let Some(Some(instance)) = instances.get(id) {
                return Ok(instance.clone());
            }
        }
        
        // Wait for an available instance
        let wait_timeout = config.wait_timeout;
        
        // Release the config lock before waiting
        drop(config);
        
        // Wait for an available instance or timeout
        let mut lock = self.instances.lock();
        let result = self.condvar.wait_for(&mut lock, wait_timeout);
        
        if result.timed_out() {
            return Err(ConcurrencyError::AcquisitionTimeout(
                self.plugin_id.clone(),
                wait_timeout.as_millis() as u64,
            ).into());
        }
        
        // Try again to get an available instance
        if let Some(id) = self.available.pop() {
            if let Some(Some(instance)) = lock.get(id) {
                return Ok(instance.clone());
            }
        }
        
        // Still no instance, create one as a last resort
        let id = self.create_instance()?;
        
        // Get the instance
        if let Some(Some(instance)) = lock.get(id) {
            return Ok(instance.clone());
        }
        
        // If we get here, something went very wrong
        Err(ConcurrencyError::InstanceCreationFailed(
            "Failed to acquire or create an instance".to_string()
        ).into())
    }
    
    /// Return an instance to the pool.
    fn return_instance(&self, id: usize) {
        // Check if we're shutting down
        if self.shutting_down.load(Ordering::SeqCst) > 0 {
            // Remove the instance
            let mut instances = self.instances.lock();
            if id < instances.len() {
                instances[id] = None;
                
                // Decrement the count
                self.count.fetch_sub(1, Ordering::SeqCst);
            }
            return;
        }
        
        // Check if the instance exists
        let instances = self.instances.lock();
        if id >= instances.len() || instances[id].is_none() {
            return;
        }
        
        // Add to available queue
        let _ = self.available.push(id);
        
        // Notify waiters
        self.condvar.notify_one();
    }
    
    /// Clean up idle instances.
    pub fn cleanup_idle(&self) -> Result<usize> {
        let config = self.config.lock();
        let idle_timeout = config.idle_timeout;
        let min_instances = config.min_instances;
        
        // Get current count
        let count = self.count.load(Ordering::SeqCst);
        if count <= min_instances {
            // Don't remove instances if we're at or below the minimum
            return Ok(0);
        }
        
        // Get idle instances
        let mut to_remove = Vec::new();
        let mut instances = self.instances.lock();
        
        for (id, instance_opt) in instances.iter().enumerate() {
            if let Some(instance) = instance_opt {
                // Check if the instance is idle
                if instance.idle_time() > idle_timeout {
                    to_remove.push(id);
                    
                    // Stop if we'd go below the minimum
                    if count - to_remove.len() <= min_instances {
                        break;
                    }
                }
            }
        }
        
        // Remove the idle instances
        for id in &to_remove {
            instances[*id] = None;
            
            // Decrement the count
            self.count.fetch_sub(1, Ordering::SeqCst);
        }
        
        Ok(to_remove.len())
    }
    
    /// Shut down the pool.
    pub fn shutdown(&self) -> Result<()> {
        // Mark as shutting down
        self.shutting_down.store(1, Ordering::SeqCst);
        
        // Clear the available queue
        while self.available.pop().is_some() {}
        
        // Remove all instances
        let mut instances = self.instances.lock();
        for instance in instances.iter_mut() {
            *instance = None;
        }
        
        // Reset the count
        self.count.store(0, Ordering::SeqCst);
        
        // Notify all waiters
        self.condvar.notify_all();
        
        Ok(())
    }
}

impl Clone for InstancePool {
    fn clone(&self) -> Self {
        Self {
            plugin_id: self.plugin_id.clone(),
            backend: self.backend.clone(),
            config: Mutex::new(self.config.lock().clone()),
            available: self.available.clone(),
            instances: Mutex::new(self.instances.lock().clone()),
            count: AtomicUsize::new(self.count.load(Ordering::SeqCst)),
            next_id: AtomicUsize::new(self.next_id.load(Ordering::SeqCst)),
            condvar: Condvar::new(),
            shutting_down: AtomicUsize::new(self.shutting_down.load(Ordering::SeqCst)),
        }
    }
}