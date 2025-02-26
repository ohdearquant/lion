//! Instance pooling for efficient resource reuse.
//!
//! Manages pools of pre-initialized instances (like WebAssembly modules)
//! to avoid the overhead of repeated initialization.

use log::{debug, info, trace};
use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for an instance pool
#[derive(Debug, Clone)]
pub struct InstancePoolConfig {
    /// Initial number of instances to pre-create
    pub initial_instances: usize,

    /// Maximum number of instances to keep in the pool
    pub max_instances: usize,

    /// Maximum age of an instance before it's recycled
    pub max_age: Duration,

    /// Maximum number of times an instance can be reused
    pub max_uses: usize,
}

impl Default for InstancePoolConfig {
    fn default() -> Self {
        Self {
            initial_instances: 5,
            max_instances: 20,
            max_age: Duration::from_secs(300), // 5 minutes
            max_uses: 100,
        }
    }
}

/// Trait for poolable instances
pub trait Poolable: Send + 'static {
    /// Create a new instance
    fn create() -> Self;

    /// Reset the instance state for reuse
    fn reset(&mut self);

    /// Check if the instance is still usable
    fn is_healthy(&self) -> bool;
}

/// A wrapper around a pooled instance with usage metadata
struct PooledInstance<T: Poolable> {
    /// The actual instance
    instance: T,

    /// When this instance was created
    created_at: Instant,

    /// Number of times this instance has been used
    use_count: usize,

    /// When this instance was last returned to the pool
    last_used_at: Instant,
}

impl<T: Poolable> PooledInstance<T> {
    /// Create a new pooled instance
    fn new(instance: T) -> Self {
        let now = Instant::now();
        Self {
            instance,
            created_at: now,
            use_count: 0,
            last_used_at: now,
        }
    }

    /// Check if the instance should be recycled based on age or usage
    fn should_recycle(&self, config: &InstancePoolConfig) -> bool {
        let age = self.created_at.elapsed();
        age > config.max_age || self.use_count >= config.max_uses
    }

    /// Increment the use count and update the last used time
    fn mark_used(&mut self) {
        self.use_count += 1;
        self.last_used_at = Instant::now();
    }
}

/// A handle to a pooled instance that returns it to the pool when dropped
pub struct InstanceHandle<T: Poolable> {
    /// The pooled instance
    instance: Option<T>,

    /// Reference to the pool this instance came from
    pool: Arc<InstancePool<T>>,
}

impl<T: Poolable> InstanceHandle<T> {
    /// Create a new instance handle
    fn new(instance: T, pool: Arc<InstancePool<T>>) -> Self {
        Self {
            instance: Some(instance),
            pool,
        }
    }

    /// Get a reference to the instance
    pub fn get(&self) -> &T {
        self.instance.as_ref().expect("Instance missing")
    }

    /// Get a mutable reference to the instance
    pub fn get_mut(&mut self) -> &mut T {
        self.instance.as_mut().expect("Instance missing")
    }

    /// Manually return the instance to the pool
    pub fn return_to_pool(mut self) {
        if let Some(instance) = self.instance.take() {
            self.pool.return_instance(instance);
        }
    }
}

impl<T: Poolable> Drop for InstanceHandle<T> {
    fn drop(&mut self) {
        if let Some(instance) = self.instance.take() {
            self.pool.return_instance(instance);
        }
    }
}

impl<T: Poolable + fmt::Debug> fmt::Debug for InstanceHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(instance) = &self.instance {
            write!(f, "InstanceHandle({:?})", instance)
        } else {
            write!(f, "InstanceHandle(returned)")
        }
    }
}

/// A pool of reusable instances
pub struct InstancePool<T: Poolable> {
    /// Available instances
    instances: Mutex<VecDeque<PooledInstance<T>>>,

    /// Configuration for this pool
    config: InstancePoolConfig,

    /// Statistics about this pool
    stats: Mutex<PoolStats>,

    /// Phantom data to ensure proper variance
    _phantom: PhantomData<T>,
}

/// Statistics about an instance pool
#[derive(Debug, Default, Clone)]
pub struct PoolStats {
    /// Total number of instances created
    pub total_created: usize,

    /// Total number of instances recycled
    pub total_recycled: usize,

    /// Total number of instance checkout operations
    pub total_checkouts: usize,

    /// Total number of instance return operations
    pub total_returns: usize,
}

impl<T: Poolable> InstancePool<T> {
    /// Create a new instance pool with the specified configuration
    pub fn new(config: InstancePoolConfig) -> Arc<Self> {
        let pool = Arc::new(Self {
            instances: Mutex::new(VecDeque::with_capacity(config.max_instances)),
            config,
            stats: Mutex::new(PoolStats::default()),
            _phantom: PhantomData,
        });

        let pool_clone = pool.clone();
        // Pre-create initial instances
        pool_clone.initialize();

        pool
    }

    /// Initialize the pool with the initial instances
    fn initialize(&self) {
        info!(
            "Initializing instance pool with {} instances",
            self.config.initial_instances
        );

        let mut instances = self.instances.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        for _ in 0..self.config.initial_instances {
            let instance = T::create();
            instances.push_back(PooledInstance::new(instance));
            stats.total_created += 1;
        }
    }

    /// Get an instance from the pool
    pub fn get_instance(&self) -> InstanceHandle<T> {
        let mut instances = self.instances.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Try to get an existing instance from the pool
        while let Some(mut pooled) = instances.pop_front() {
            // Check if the instance needs to be recycled
            if pooled.should_recycle(&self.config) {
                trace!("Recycling old instance");
                stats.total_recycled += 1;
                continue;
            }

            // Check if the instance is healthy
            if !pooled.instance.is_healthy() {
                trace!("Discarding unhealthy instance");
                stats.total_recycled += 1;
                continue;
            }

            // Mark the instance as used
            pooled.mark_used();
            stats.total_checkouts += 1;

            return InstanceHandle::new(pooled.instance, Arc::new(self.clone()));
        }

        // If no instance available, create a new one
        debug!("Creating new instance on demand");
        let instance = T::create();
        stats.total_created += 1;
        stats.total_checkouts += 1;

        InstanceHandle::new(instance, Arc::new(self.clone()))
    }

    /// Return an instance to the pool
    fn return_instance(&self, mut instance: T) {
        let mut instances = self.instances.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        stats.total_returns += 1;

        // Reset the instance state
        instance.reset();

        // Check if the pool is full
        if instances.len() >= self.config.max_instances {
            // Pool is full, discard the oldest instance
            if let Some(_) = instances.pop_front() {
                trace!("Discarding oldest instance to make room");
                stats.total_recycled += 1;
            }
        }

        // Add the instance back to the pool
        instances.push_back(PooledInstance::new(instance));
    }

    /// Get the current statistics for this pool
    pub fn get_stats(&self) -> PoolStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }

    /// Get the current number of available instances
    pub fn available_count(&self) -> usize {
        let instances = self.instances.lock().unwrap();
        instances.len()
    }
}

impl<T: Poolable> Clone for InstancePool<T> {
    fn clone(&self) -> Self {
        Self {
            instances: Mutex::new(VecDeque::new()),
            config: self.config.clone(),
            stats: Mutex::new(PoolStats::default()),
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[derive(Debug)]
    struct TestInstance {
        id: usize,
        healthy: bool,
    }

    static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

    impl Poolable for TestInstance {
        fn create() -> Self {
            let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Self { id, healthy: true }
        }

        fn reset(&mut self) {
            // No-op for test
        }

        fn is_healthy(&self) -> bool {
            self.healthy
        }
    }

    #[test]
    fn test_instance_pool_basic() {
        let config = InstancePoolConfig {
            initial_instances: 2,
            max_instances: 5,
            max_age: Duration::from_secs(10),
            max_uses: 3,
        };

        let pool = InstancePool::<TestInstance>::new(config);

        // Initial count should match configuration
        assert_eq!(pool.available_count(), 2);

        // Get an instance
        let handle1 = pool.get_instance();
        assert_eq!(pool.available_count(), 1);

        // Get another instance
        let handle2 = pool.get_instance();
        assert_eq!(pool.available_count(), 0);

        // Return one instance
        drop(handle1);
        assert_eq!(pool.available_count(), 1);

        // Return the other instance
        drop(handle2);
        assert_eq!(pool.available_count(), 2);
    }

    #[test]
    fn test_instance_recycling() {
        let config = InstancePoolConfig {
            initial_instances: 1,
            max_instances: 1,
            max_age: Duration::from_millis(10), // Very short age for testing
            max_uses: 10,
        };

        let pool = InstancePool::<TestInstance>::new(config);

        // Get the initial instance
        let handle = pool.get_instance();
        let id1 = handle.get().id;
        drop(handle);

        // Wait for the age to exceed max_age
        thread::sleep(Duration::from_millis(20));

        // Get another instance - should be a new one
        let handle = pool.get_instance();
        let id2 = handle.get().id;

        // IDs should be different due to recycling
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_instance_max_uses() {
        let config = InstancePoolConfig {
            initial_instances: 1,
            max_instances: 1,
            max_age: Duration::from_secs(10),
            max_uses: 2, // Only allow 2 uses
        };

        let pool = InstancePool::<TestInstance>::new(config);

        // Get the instance twice - should be the same one
        let handle1 = pool.get_instance();
        let id1 = handle1.get().id;
        drop(handle1);

        let handle2 = pool.get_instance();
        let id2 = handle2.get().id;
        assert_eq!(id1, id2);
        drop(handle2);

        // Third use should get a new instance
        let handle3 = pool.get_instance();
        let id3 = handle3.get().id;
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_pool_stats() {
        let config = InstancePoolConfig::default();
        let pool = InstancePool::<TestInstance>::new(config);

        // Initial stats
        let stats = pool.get_stats();
        assert_eq!(stats.total_created, 5); // Default initial_instances
        assert_eq!(stats.total_checkouts, 0);

        // Checkout and return instances
        let handle1 = pool.get_instance();
        let handle2 = pool.get_instance();
        drop(handle1);

        // Updated stats
        let stats = pool.get_stats();
        assert_eq!(stats.total_checkouts, 2);
        assert_eq!(stats.total_returns, 1);

        // Return last instance
        drop(handle2);
        let stats = pool.get_stats();
        assert_eq!(stats.total_returns, 2);
    }

    #[test]
    fn test_unhealthy_instance() {
        let config = InstancePoolConfig::default();
        let pool = InstancePool::<TestInstance>::new(config);

        // Get an instance and mark it as unhealthy
        let mut handle = pool.get_instance();
        handle.get_mut().healthy = false;

        // Return it to the pool
        drop(handle);

        // Stats before getting a new instance
        let stats_before = pool.get_stats();

        // Get another instance - should detect and discard the unhealthy one
        let _handle = pool.get_instance();

        // Stats after - should have recycled one instance
        let stats_after = pool.get_stats();
        assert_eq!(stats_after.total_recycled, stats_before.total_recycled + 1);
    }
}
