//! Resource pooling for reusable resources like connections, buffers, etc.
//!
//! Provides efficient pooling and lifecycle management for expensive resources.

use log::{debug, info, trace, warn};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Error returned when a resource cannot be acquired from the pool
#[derive(Error, Debug)]
pub enum ResourcePoolError {
    /// The pool is exhausted (no resources available)
    #[error("resource pool exhausted")]
    PoolExhausted,

    /// A timeout occurred while waiting for a resource
    #[error("timeout waiting for resource")]
    Timeout,

    /// The pool is shut down
    #[error("resource pool is shut down")]
    PoolShutdown,

    /// Error creating a new resource
    #[error("failed to create resource: {0}")]
    CreationFailed(String),
}

/// Configuration for a resource pool
#[derive(Debug, Clone)]
pub struct ResourcePoolConfig {
    /// Initial number of resources to pre-create
    pub initial_size: usize,

    /// Maximum number of resources to allow
    pub max_size: usize,

    /// Maximum time to keep an idle resource
    pub max_idle_time: Duration,

    /// Default timeout when acquiring a resource
    pub acquire_timeout: Duration,

    /// Interval for checking resource health
    pub health_check_interval: Duration,
}

impl Default for ResourcePoolConfig {
    fn default() -> Self {
        Self {
            initial_size: 5,
            max_size: 20,
            max_idle_time: Duration::from_secs(60),
            acquire_timeout: Duration::from_secs(3),
            health_check_interval: Duration::from_secs(30),
        }
    }
}

/// A trait for resources that can be pooled
pub trait Resource: Send + Sync + 'static {
    /// Create a new instance of the resource
    fn create() -> Result<Self, String>
    where
        Self: Sized;

    /// Reset the resource for reuse
    fn reset(&mut self) -> bool;

    /// Check if the resource is still valid and healthy
    fn is_valid(&self) -> bool;

    /// Close the resource when it's no longer needed
    fn close(&mut self);
}

/// A handle to a resource from the pool
pub struct ResourceHandle<R: Resource> {
    /// The resource itself
    resource: Option<R>,

    /// Reference to the pool this resource belongs to
    pool: Weak<ResourcePool<R>>,

    /// When this resource was acquired
    acquired_at: Instant,
}

impl<R: Resource> ResourceHandle<R> {
    /// Create a new resource handle
    #[allow(dead_code)]
    fn new(resource: R, pool: Arc<ResourcePool<R>>) -> Self {
        Self {
            resource: Some(resource),
            pool: Arc::downgrade(&pool),
            acquired_at: Instant::now(),
        }
    }

    /// Get a reference to the resource
    pub fn get(&self) -> &R {
        self.resource.as_ref().expect("Resource missing")
    }

    /// Get a mutable reference to the resource
    pub fn get_mut(&mut self) -> &mut R {
        self.resource.as_mut().expect("Resource missing")
    }

    /// Return the resource to the pool before the handle is dropped
    pub fn return_to_pool(mut self) {
        if let Some(resource) = self.resource.take() {
            if let Some(pool) = self.pool.upgrade() {
                pool.return_resource(resource);
            } else {
                // Pool no longer exists, close the resource
                let mut res = resource;
                res.close();
            }
        }
    }

    /// Get the time since this resource was acquired
    pub fn held_duration(&self) -> Duration {
        self.acquired_at.elapsed()
    }
}

impl<R: Resource> Drop for ResourceHandle<R> {
    fn drop(&mut self) {
        if let Some(resource) = self.resource.take() {
            if let Some(pool) = self.pool.upgrade() {
                pool.return_resource(resource);
            } else {
                // Pool no longer exists, close the resource
                let mut res = resource;
                res.close();
            }
        }
    }
}

#[allow(dead_code)]
struct PooledResource<R: Resource> {
    /// The resource itself
    resource: R,

    /// When this resource was created
    created_at: Instant,

    /// When this resource was last returned to the pool
    last_used_at: Instant,

    /// Last time this resource was health-checked
    last_checked_at: Instant,
}

impl<R: Resource> PooledResource<R> {
    /// Create a new pooled resource
    fn new(resource: R) -> Self {
        let now = Instant::now();
        Self {
            resource,
            created_at: now,
            last_used_at: now,
            last_checked_at: now,
        }
    }

    /// Check if this resource has been idle for too long
    fn is_idle_timeout(&self, max_idle_time: Duration) -> bool {
        self.last_used_at.elapsed() > max_idle_time
    }

    /// Check if this resource is due for a health check
    fn is_health_check_due(&self, interval: Duration) -> bool {
        self.last_checked_at.elapsed() > interval
    }

    /// Perform a health check on this resource
    fn check_health(&mut self) -> bool {
        self.last_checked_at = Instant::now();
        self.resource.is_valid()
    }
}

/// A pool of reusable resources
pub struct ResourcePool<R: Resource> {
    /// Available resources
    resources: Mutex<VecDeque<PooledResource<R>>>,

    /// Current size of the pool (available + in use)
    size: Mutex<usize>,

    /// Configuration for this pool
    config: ResourcePoolConfig,

    /// Whether this pool is shut down
    shutdown: Mutex<bool>,

    /// Phantom data to ensure proper variance
    _phantom: PhantomData<R>,
}

impl<R: Resource> ResourcePool<R> {
    /// Create a new resource pool with the specified configuration
    pub fn new(config: ResourcePoolConfig) -> Arc<Self> {
        let pool = Arc::new(Self {
            resources: Mutex::new(VecDeque::with_capacity(config.max_size)),
            size: Mutex::new(0),
            config,
            shutdown: Mutex::new(false),
            _phantom: PhantomData,
        });

        // Initialize the pool
        {
            let pool_clone = Arc::clone(&pool);
            pool_clone.initialize();
        }

        pool
    }

    /// Initialize the pool with the initial resources
    fn initialize(&self) {
        info!(
            "Initializing resource pool with {} resources",
            self.config.initial_size
        );

        let mut resources = self.resources.lock().unwrap();
        let mut size = self.size.lock().unwrap();

        for _ in 0..self.config.initial_size {
            match R::create() {
                Ok(resource) => {
                    resources.push_back(PooledResource::new(resource));
                    *size += 1;
                }
                Err(e) => {
                    warn!("Failed to create resource during initialization: {}", e);
                }
            }
        }

        debug!(
            "Resource pool initialized with {} resources",
            resources.len()
        );
    }

    /// Acquire a resource from the pool with the default timeout
    pub fn acquire(self: &Arc<Self>) -> Result<ResourceHandle<R>, ResourcePoolError> {
        self.acquire_with_timeout(self.config.acquire_timeout)
    }

    /// Acquire a resource with a specified timeout
    pub fn acquire_with_timeout(
        self: &Arc<Self>,
        timeout: Duration,
    ) -> Result<ResourceHandle<R>, ResourcePoolError> {
        let start_time = Instant::now();

        // Check if the pool is shut down
        if *self.shutdown.lock().unwrap() {
            return Err(ResourcePoolError::PoolShutdown);
        }

        while start_time.elapsed() < timeout {
            // Try to get a resource
            if let Some(resource) = self.get_resource() {
                return Ok(ResourceHandle {
                    resource: Some(resource),
                    pool: Arc::downgrade(&self),
                    acquired_at: Instant::now(),
                });
            }

            // If we can create a new resource, do so
            let current_size = *self.size.lock().unwrap();
            if current_size < self.config.max_size {
                // Create a new resource
                match R::create() {
                    Ok(resource) => {
                        *self.size.lock().unwrap() += 1;
                        return Ok(ResourceHandle {
                            resource: Some(resource),
                            pool: Arc::downgrade(&self),
                            acquired_at: Instant::now(),
                        });
                    }
                    Err(e) => {
                        return Err(ResourcePoolError::CreationFailed(e));
                    }
                }
            }

            // Wait a bit before trying again
            std::thread::sleep(Duration::from_millis(10));
        }

        // Timeout occurred
        Err(ResourcePoolError::Timeout)
    }

    /// Try to acquire a resource without waiting
    pub fn try_acquire(self: &Arc<Self>) -> Result<ResourceHandle<R>, ResourcePoolError> {
        // Check if the pool is shut down
        if *self.shutdown.lock().unwrap() {
            return Err(ResourcePoolError::PoolShutdown);
        }

        // Try to get a resource
        if let Some(resource) = self.get_resource() {
            return Ok(ResourceHandle {
                resource: Some(resource),
                pool: Arc::downgrade(&self),
                acquired_at: Instant::now(),
            });
        }

        // If we can create a new resource, do so
        let current_size = *self.size.lock().unwrap();
        if current_size < self.config.max_size {
            // Create a new resource
            match R::create() {
                Ok(resource) => {
                    *self.size.lock().unwrap() += 1;
                    return Ok(ResourceHandle {
                        resource: Some(resource),
                        pool: Arc::downgrade(&self),
                        acquired_at: Instant::now(),
                    });
                }
                Err(e) => {
                    return Err(ResourcePoolError::CreationFailed(e));
                }
            }
        }

        // No resources available
        Err(ResourcePoolError::PoolExhausted)
    }

    /// Get a resource from the pool, performing cleanup as needed
    fn get_resource(&self) -> Option<R> {
        // Check if the pool is shut down first, before acquiring any locks
        if *self.shutdown.lock().unwrap() {
            trace!("Pool is shut down, no resources available");
            return None;
        }

        let mut resources = self.resources.lock().unwrap();

        while let Some(mut pooled) = resources.pop_front() {
            // Check if the resource has been idle for too long
            if pooled.is_idle_timeout(self.config.max_idle_time) {
                trace!("Removing idle resource");
                pooled.resource.close();
                *self.size.lock().unwrap() -= 1;
                continue;
            }

            // Check if the resource needs a health check
            if pooled.is_health_check_due(self.config.health_check_interval) {
                if !pooled.check_health() {
                    trace!("Removing unhealthy resource");
                    pooled.resource.close();
                    *self.size.lock().unwrap() -= 1;
                    continue;
                }
            }

            // Reset the resource for reuse
            if !pooled.resource.reset() {
                trace!("Resource reset failed, creating new resource");
                pooled.resource.close();
                *self.size.lock().unwrap() -= 1;
                continue;
            }

            // Resource is good to use
            return Some(pooled.resource);
        }

        None
    }

    /// Return a resource to the pool
    fn return_resource(&self, resource: R) {
        if *self.shutdown.lock().unwrap() {
            // Pool is shut down, close the resource immediately
            // without trying to acquire any other locks
            let mut res = resource;
            res.close();
            return;
        }

        // Use a block to ensure locks are released in the right order
        {
            let mut resources = self.resources.lock().unwrap();

            // Check if the resource is valid
            if !resource.is_valid() {
                trace!("Returned resource is invalid, discarding");
                let mut res = resource;
                res.close();
                *self.size.lock().unwrap() -= 1;
                return;
            }

            // Add the resource back to the pool
            resources.push_back(PooledResource::new(resource));
        }
    }

    /// Shut down the pool, closing all resources
    pub fn shutdown(&self) {
        info!("Shutting down resource pool...");

        // First mark the pool as shut down to prevent new acquisitions
        // and to signal to ResourceHandle::drop that they should close resources
        *self.shutdown.lock().unwrap() = true;

        // Wait a bit to ensure in-flight resource returns complete
        std::thread::sleep(Duration::from_millis(50));

        // Now close all resources that are in the pool
        {
            let mut resources = self.resources.lock().unwrap();
            let mut size = self.size.lock().unwrap();

            while let Some(mut pooled) = resources.pop_front() {
                pooled.resource.close();
                *size -= 1;
            }
        }

        info!("Resource pool shutdown complete.");
    }

    /// Get the current number of available resources
    pub fn available_count(&self) -> usize {
        let resources = self.resources.lock().unwrap();
        resources.len()
    }

    /// Get the total number of resources (available + in use)
    pub fn total_count(&self) -> usize {
        *self.size.lock().unwrap()
    }

    /// Perform maintenance on the pool (remove idle/unhealthy resources)
    pub fn maintenance(&self) {
        let mut resources = self.resources.lock().unwrap();
        let mut i = 0;

        while i < resources.len() {
            // Check if the resource has been idle for too long
            if resources[i].is_idle_timeout(self.config.max_idle_time) {
                trace!("Removing idle resource during maintenance");
                let mut pooled = resources.remove(i).unwrap();
                pooled.resource.close();
                *self.size.lock().unwrap() -= 1;
                continue;
            }

            // Check if the resource needs a health check
            if resources[i].is_health_check_due(self.config.health_check_interval) {
                if !resources[i].check_health() {
                    trace!("Removing unhealthy resource during maintenance");
                    let mut pooled = resources.remove(i).unwrap();
                    pooled.resource.close();
                    *self.size.lock().unwrap() -= 1;
                    continue;
                }
            }

            // Move to the next resource
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // A simple test resource implementation
    #[allow(dead_code)]
    struct TestResource {
        id: usize,
        created_count: Arc<AtomicUsize>,
        valid: bool,
        closed_count: Arc<AtomicUsize>,
    }

    impl Resource for TestResource {
        fn create() -> Result<Self, String> {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

            let created_count = Arc::new(AtomicUsize::new(1));
            let closed_count = Arc::new(AtomicUsize::new(0));

            Ok(Self {
                id,
                valid: true,
                created_count,
                closed_count,
            })
        }

        fn reset(&mut self) -> bool {
            self.valid
        }

        fn is_valid(&self) -> bool {
            self.valid
        }

        fn close(&mut self) {
            self.closed_count.fetch_add(1, Ordering::Relaxed);
            self.valid = false;
        }
    }

    #[test]
    fn test_resource_pool_basic() {
        let config = ResourcePoolConfig {
            initial_size: 2,
            max_size: 5,
            max_idle_time: Duration::from_secs(60),
            acquire_timeout: Duration::from_secs(1),
            health_check_interval: Duration::from_secs(30),
        };

        let pool = ResourcePool::<TestResource>::new(config);

        // Add a small delay to ensure pool initialization is complete
        std::thread::sleep(Duration::from_millis(50));

        // Check initial size
        assert_eq!(pool.total_count(), 2);
        assert_eq!(pool.available_count(), 2);

        // Acquire a resource
        let handle1 = pool.acquire().unwrap();
        assert_eq!(pool.available_count(), 1);

        // Acquire another resource
        let handle2 = pool.acquire().unwrap();
        assert_eq!(pool.available_count(), 0);

        // Return one resource
        drop(handle1);

        // Add a small delay to ensure resource is properly returned
        std::thread::sleep(Duration::from_millis(10));

        assert_eq!(pool.available_count(), 1);

        // Return the other resource
        drop(handle2);

        // Add a small delay to ensure resource is properly returned
        std::thread::sleep(Duration::from_millis(10));

        assert_eq!(pool.available_count(), 2);
    }

    #[test]
    fn test_resource_pool_growth() {
        let config = ResourcePoolConfig {
            initial_size: 1,
            max_size: 3,
            max_idle_time: Duration::from_secs(60),
            acquire_timeout: Duration::from_secs(1),
            health_check_interval: Duration::from_secs(30),
        };

        let pool = ResourcePool::<TestResource>::new(config);

        // Add a small delay to ensure pool initialization is complete
        std::thread::sleep(Duration::from_millis(50));

        // Check initial size
        assert_eq!(pool.total_count(), 1);

        // Acquire resources until we hit max size
        let handle1 = pool.acquire().unwrap();
        assert_eq!(pool.total_count(), 1);

        let handle2 = pool.acquire().unwrap();
        assert_eq!(pool.total_count(), 2);

        let handle3 = pool.acquire().unwrap();
        assert_eq!(pool.total_count(), 3);

        // Should return pool exhausted now
        let result = pool.try_acquire();
        assert!(matches!(result, Err(ResourcePoolError::PoolExhausted)));

        // Return a resource
        drop(handle1);

        // Add a small delay to ensure resource is returned to the pool
        std::thread::sleep(Duration::from_millis(10));

        // Should be able to acquire again
        let _handle4 = pool
            .acquire_with_timeout(Duration::from_millis(500))
            .unwrap();

        // Clean up
        drop(handle2);
        drop(handle3);
    }

    #[test]
    fn test_resource_invalid() {
        let config = ResourcePoolConfig::default();
        let pool = ResourcePool::<TestResource>::new(config);

        // Add a small delay to ensure pool initialization is complete
        std::thread::sleep(Duration::from_millis(50));

        // Get a resource and invalidate it
        let mut handle = pool.acquire().unwrap();
        handle.get_mut().valid = false;

        // Return the invalid resource
        drop(handle);

        // Add a small delay to ensure resource is properly processed
        std::thread::sleep(Duration::from_millis(10));

        // Pool should have discarded the invalid resource
        assert_eq!(pool.total_count(), pool.config.initial_size - 1);
    }

    #[test]
    fn test_resource_pool_shutdown() {
        println!("Starting test_resource_pool_shutdown");
        let config = ResourcePoolConfig::default();
        let pool = ResourcePool::<TestResource>::new(config);

        // Add a small delay to ensure pool initialization is complete
        std::thread::sleep(Duration::from_millis(50));

        // Get some resources
        {
            println!("Acquiring resources");
            let handle1 = pool.acquire().unwrap();
            let handle2 = pool.acquire().unwrap();

            println!("Resources acquired, dropping handles");
            // Explicitly drop handles in this scope
            drop(handle1);
            drop(handle2);

            // Add a longer delay to ensure resources are properly processed
            println!("Waiting for resources to be returned to pool");
            std::thread::sleep(Duration::from_millis(100));
        }

        println!("Calling pool.shutdown()");
        // Shutdown the pool
        pool.shutdown();

        println!("Testing acquire after shutdown");
        // Try to acquire after shutdown
        let result = pool.acquire();
        assert!(matches!(result, Err(ResourcePoolError::PoolShutdown)));

        println!("Checking final pool state");

        // All resources should be closed
        assert_eq!(pool.total_count(), 0);
        assert_eq!(pool.available_count(), 0);
    }

    #[test]
    fn test_resource_pool_maintenance() {
        let config = ResourcePoolConfig {
            initial_size: 3,
            max_size: 5,
            max_idle_time: Duration::from_millis(10), // Very short for testing
            acquire_timeout: Duration::from_secs(1),
            health_check_interval: Duration::from_secs(30),
        };

        let pool = ResourcePool::<TestResource>::new(config);

        // Add a small delay to ensure pool initialization is complete
        std::thread::sleep(Duration::from_millis(50));

        // Wait for idle timeout
        std::thread::sleep(Duration::from_millis(20));

        // Run maintenance
        pool.maintenance();

        // All resources should have been removed due to idle timeout
        assert_eq!(pool.total_count(), 0);
    }
}
