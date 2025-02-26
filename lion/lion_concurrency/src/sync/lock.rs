//! Custom lock implementations with extended capabilities.
//!
//! Provides locks with timeouts, statistics, and diagnostics capabilities.

use log::{trace, warn};
use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Error when acquiring a lock
#[derive(Error, Debug)]
pub enum LockError {
    /// The lock could not be acquired within the specified timeout
    #[error("lock acquisition timed out after {0:?}")]
    Timeout(Duration),

    /// The lock is poisoned and cannot be acquired
    #[error("lock is poisoned")]
    Poisoned,

    /// The lock is deadlocked
    #[error("deadlock detected")]
    Deadlock,
}

/// Statistics about lock usage
#[derive(Debug, Default, Clone)]
pub struct LockStats {
    /// Number of successful lock acquisitions
    pub acquisition_count: usize,

    /// Number of failed lock acquisition attempts
    pub failed_count: usize,

    /// Total time spent waiting for the lock (microseconds)
    pub total_wait_time_us: u64,

    /// Total time the lock was held (microseconds)
    pub total_hold_time_us: u64,

    /// Maximum time spent waiting for the lock (microseconds)
    pub max_wait_time_us: u64,

    /// Maximum time the lock was held (microseconds)
    pub max_hold_time_us: u64,

    /// Number of times a deadlock was detected
    pub deadlock_count: usize,
}

/// A mutex with timeout and statistics
pub struct TrackedMutex<T> {
    /// The underlying mutex
    mutex: Mutex<T>,

    /// Statistics about lock usage
    stats: Arc<TrackedMutexStats>,

    /// Name of this mutex for debugging
    name: Option<String>,
}

/// Statistics for TrackedMutex
#[derive(Debug, Default)]
struct TrackedMutexStats {
    /// Number of successful lock acquisitions
    acquisition_count: AtomicUsize,

    /// Number of failed lock acquisition attempts
    failed_count: AtomicUsize,

    /// Total time spent waiting for the lock (microseconds)
    total_wait_time_us: AtomicUsize,

    /// Total time the lock was held (microseconds)
    total_hold_time_us: AtomicUsize,

    /// Maximum time spent waiting for the lock (microseconds)
    max_wait_time_us: AtomicUsize,

    /// Maximum time the lock was held (microseconds)
    max_hold_time_us: AtomicUsize,

    /// Number of times a deadlock was detected
    deadlock_count: AtomicUsize,
}

/// A guard for a TrackedMutex
pub struct TrackedMutexGuard<'a, T> {
    /// The underlying mutex guard
    guard: MutexGuard<'a, T>,

    /// When the lock was acquired
    acquired_at: Instant,

    /// Statistics for this mutex
    stats: Arc<TrackedMutexStats>,

    /// Name of the mutex
    name: Option<String>,
}

impl<T> TrackedMutex<T> {
    /// Create a new tracked mutex
    pub fn new(value: T) -> Self {
        Self {
            mutex: Mutex::new(value),
            stats: Arc::new(TrackedMutexStats::default()),
            name: None,
        }
    }

    /// Create a new tracked mutex with a name for debugging
    pub fn with_name(value: T, name: impl Into<String>) -> Self {
        Self {
            mutex: Mutex::new(value),
            stats: Arc::new(TrackedMutexStats::default()),
            name: Some(name.into()),
        }
    }

    /// Lock the mutex
    pub fn lock(&self) -> TrackedMutexGuard<'_, T> {
        let start = Instant::now();
        let guard = self.mutex.lock();
        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_wait_time(wait_time_us);

        // Update acquisition count
        self.stats.acquisition_count.fetch_add(1, Ordering::Relaxed);

        trace!(
            "Lock acquired: {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        TrackedMutexGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.stats),
            name: self.name.clone(),
        }
    }

    /// Try to lock the mutex
    pub fn try_lock(&self) -> Option<TrackedMutexGuard<'_, T>> {
        let start = Instant::now();
        let guard = self.mutex.try_lock()?;
        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_wait_time(wait_time_us);

        // Update acquisition count
        self.stats.acquisition_count.fetch_add(1, Ordering::Relaxed);

        trace!(
            "Lock acquired (try_lock): {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        Some(TrackedMutexGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.stats),
            name: self.name.clone(),
        })
    }

    /// Try to lock the mutex with a timeout
    pub fn try_lock_for(&self, timeout: Duration) -> Result<TrackedMutexGuard<'_, T>, LockError> {
        let start = Instant::now();

        // Try to acquire the lock with the timeout
        let guard = self.mutex.try_lock_for(timeout).ok_or_else(|| {
            // Update failed count if we timed out
            self.stats.failed_count.fetch_add(1, Ordering::Relaxed);

            let wait_time = start.elapsed();
            warn!(
                "Lock timeout: {} (waited: {:.2}ms, timeout: {:.2}ms)",
                self.name.as_deref().unwrap_or("unnamed"),
                wait_time.as_secs_f64() * 1000.0,
                timeout.as_secs_f64() * 1000.0
            );

            LockError::Timeout(timeout)
        })?;

        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_wait_time(wait_time_us);

        // Update acquisition count
        self.stats.acquisition_count.fetch_add(1, Ordering::Relaxed);

        trace!(
            "Lock acquired (with timeout): {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        Ok(TrackedMutexGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.stats),
            name: self.name.clone(),
        })
    }

    /// Get the statistics for this mutex
    pub fn stats(&self) -> LockStats {
        LockStats {
            acquisition_count: self.stats.acquisition_count.load(Ordering::Relaxed),
            failed_count: self.stats.failed_count.load(Ordering::Relaxed),
            total_wait_time_us: self.stats.total_wait_time_us.load(Ordering::Relaxed) as u64,
            total_hold_time_us: self.stats.total_hold_time_us.load(Ordering::Relaxed) as u64,
            max_wait_time_us: self.stats.max_wait_time_us.load(Ordering::Relaxed) as u64,
            max_hold_time_us: self.stats.max_hold_time_us.load(Ordering::Relaxed) as u64,
            deadlock_count: self.stats.deadlock_count.load(Ordering::Relaxed),
        }
    }

    /// Reset the statistics
    pub fn reset_stats(&self) {
        self.stats.acquisition_count.store(0, Ordering::Relaxed);
        self.stats.failed_count.store(0, Ordering::Relaxed);
        self.stats.total_wait_time_us.store(0, Ordering::Relaxed);
        self.stats.total_hold_time_us.store(0, Ordering::Relaxed);
        self.stats.max_wait_time_us.store(0, Ordering::Relaxed);
        self.stats.max_hold_time_us.store(0, Ordering::Relaxed);
        self.stats.deadlock_count.store(0, Ordering::Relaxed);
    }

    /// Get the name of this mutex
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the contention factor (higher means more contention)
    pub fn contention_factor(&self) -> f64 {
        let stats = self.stats();

        if stats.acquisition_count == 0 {
            return 0.0;
        }

        let avg_wait_time = stats.total_wait_time_us as f64 / stats.acquisition_count as f64;
        let avg_hold_time = stats.total_hold_time_us as f64 / stats.acquisition_count as f64;

        if avg_hold_time == 0.0 {
            return 0.0;
        }

        avg_wait_time / avg_hold_time
    }

    /// Helper method to update max wait time atomically
    fn update_max_wait_time(&self, wait_time_us: usize) {
        let mut current_max = self.stats.max_wait_time_us.load(Ordering::Relaxed);

        while wait_time_us > current_max {
            match self.stats.max_wait_time_us.compare_exchange(
                current_max,
                wait_time_us,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }
}

impl<T: Default> Clone for TrackedMutex<T> {
    fn clone(&self) -> Self {
        Self {
            mutex: Mutex::new(Default::default()),
            stats: Arc::clone(&self.stats),
            name: self.name.clone(),
        }
    }
}

impl<T> Default for TrackedMutex<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<'a, T> Drop for TrackedMutexGuard<'a, T> {
    fn drop(&mut self) {
        // Calculate how long the lock was held
        let hold_time = self.acquired_at.elapsed();
        let hold_time_us = hold_time.as_micros() as usize;

        // Update hold time statistics
        self.stats
            .total_hold_time_us
            .fetch_add(hold_time_us, Ordering::Relaxed);

        // Update max hold time atomically
        let mut current_max = self.stats.max_hold_time_us.load(Ordering::Relaxed);

        while hold_time_us > current_max {
            match self.stats.max_hold_time_us.compare_exchange(
                current_max,
                hold_time_us,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }

        trace!(
            "Lock released: {} (held for: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            hold_time.as_secs_f64() * 1000.0
        );
    }
}

impl<'a, T> std::ops::Deref for TrackedMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<'a, T> std::ops::DerefMut for TrackedMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

/// A read-write lock with timeout and statistics
pub struct TrackedRwLock<T> {
    /// The underlying RwLock
    rwlock: RwLock<T>,

    /// Statistics about lock usage
    read_stats: Arc<TrackedMutexStats>,

    /// Statistics about write lock usage
    write_stats: Arc<TrackedMutexStats>,

    /// Name of this lock for debugging
    name: Option<String>,
}

/// A read guard for a TrackedRwLock
pub struct TrackedRwLockReadGuard<'a, T> {
    /// The underlying read guard
    guard: RwLockReadGuard<'a, T>,

    /// When the lock was acquired
    acquired_at: Instant,

    /// Statistics for this lock
    stats: Arc<TrackedMutexStats>,

    /// Name of the lock
    name: Option<String>,
}

/// A write guard for a TrackedRwLock
pub struct TrackedRwLockWriteGuard<'a, T> {
    /// The underlying write guard
    guard: RwLockWriteGuard<'a, T>,

    /// When the lock was acquired
    acquired_at: Instant,

    /// Statistics for this lock
    stats: Arc<TrackedMutexStats>,

    /// Name of the lock
    name: Option<String>,
}

impl<T> TrackedRwLock<T> {
    /// Create a new tracked read-write lock
    pub fn new(value: T) -> Self {
        Self {
            rwlock: RwLock::new(value),
            read_stats: Arc::new(TrackedMutexStats::default()),
            write_stats: Arc::new(TrackedMutexStats::default()),
            name: None,
        }
    }

    /// Create a new tracked read-write lock with a name for debugging
    pub fn with_name(value: T, name: impl Into<String>) -> Self {
        Self {
            rwlock: RwLock::new(value),
            read_stats: Arc::new(TrackedMutexStats::default()),
            write_stats: Arc::new(TrackedMutexStats::default()),
            name: Some(name.into()),
        }
    }

    /// Acquire a read lock
    pub fn read(&self) -> TrackedRwLockReadGuard<'_, T> {
        let start = Instant::now();
        let guard = self.rwlock.read();
        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.read_stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_read_wait_time(wait_time_us);

        // Update acquisition count
        self.read_stats
            .acquisition_count
            .fetch_add(1, Ordering::Relaxed);

        trace!(
            "Read lock acquired: {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        TrackedRwLockReadGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.read_stats),
            name: self.name.clone(),
        }
    }

    /// Try to acquire a read lock
    pub fn try_read(&self) -> Option<TrackedRwLockReadGuard<'_, T>> {
        let start = Instant::now();
        let guard = self.rwlock.try_read()?;
        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.read_stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_read_wait_time(wait_time_us);

        // Update acquisition count
        self.read_stats
            .acquisition_count
            .fetch_add(1, Ordering::Relaxed);

        trace!(
            "Read lock acquired (try_read): {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        Some(TrackedRwLockReadGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.read_stats),
            name: self.name.clone(),
        })
    }

    /// Try to acquire a read lock with a timeout
    pub fn try_read_for(
        &self,
        timeout: Duration,
    ) -> Result<TrackedRwLockReadGuard<'_, T>, LockError> {
        let start = Instant::now();

        // Try to acquire the lock with the timeout
        let guard = self.rwlock.try_read_for(timeout).ok_or_else(|| {
            // Update failed count if we timed out
            self.read_stats.failed_count.fetch_add(1, Ordering::Relaxed);

            let wait_time = start.elapsed();
            warn!(
                "Read lock timeout: {} (waited: {:.2}ms, timeout: {:.2}ms)",
                self.name.as_deref().unwrap_or("unnamed"),
                wait_time.as_secs_f64() * 1000.0,
                timeout.as_secs_f64() * 1000.0
            );

            LockError::Timeout(timeout)
        })?;

        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.read_stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_read_wait_time(wait_time_us);

        // Update acquisition count
        self.read_stats
            .acquisition_count
            .fetch_add(1, Ordering::Relaxed);

        trace!(
            "Read lock acquired (with timeout): {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        Ok(TrackedRwLockReadGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.read_stats),
            name: self.name.clone(),
        })
    }

    /// Acquire a write lock
    pub fn write(&self) -> TrackedRwLockWriteGuard<'_, T> {
        let start = Instant::now();
        let guard = self.rwlock.write();
        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.write_stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_write_wait_time(wait_time_us);

        // Update acquisition count
        self.write_stats
            .acquisition_count
            .fetch_add(1, Ordering::Relaxed);

        trace!(
            "Write lock acquired: {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        TrackedRwLockWriteGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.write_stats),
            name: self.name.clone(),
        }
    }

    /// Try to acquire a write lock
    pub fn try_write(&self) -> Option<TrackedRwLockWriteGuard<'_, T>> {
        let start = Instant::now();
        let guard = self.rwlock.try_write()?;
        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.write_stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_write_wait_time(wait_time_us);

        // Update acquisition count
        self.write_stats
            .acquisition_count
            .fetch_add(1, Ordering::Relaxed);

        trace!(
            "Write lock acquired (try_write): {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        Some(TrackedRwLockWriteGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.write_stats),
            name: self.name.clone(),
        })
    }

    /// Try to acquire a write lock with a timeout
    pub fn try_write_for(
        &self,
        timeout: Duration,
    ) -> Result<TrackedRwLockWriteGuard<'_, T>, LockError> {
        let start = Instant::now();

        // Try to acquire the lock with the timeout
        let guard = self.rwlock.try_write_for(timeout).ok_or_else(|| {
            // Update failed count if we timed out
            self.write_stats
                .failed_count
                .fetch_add(1, Ordering::Relaxed);

            let wait_time = start.elapsed();
            warn!(
                "Write lock timeout: {} (waited: {:.2}ms, timeout: {:.2}ms)",
                self.name.as_deref().unwrap_or("unnamed"),
                wait_time.as_secs_f64() * 1000.0,
                timeout.as_secs_f64() * 1000.0
            );

            LockError::Timeout(timeout)
        })?;

        let wait_time = start.elapsed();

        // Update wait time statistics
        let wait_time_us = wait_time.as_micros() as usize;
        self.write_stats
            .total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
        self.update_max_write_wait_time(wait_time_us);

        // Update acquisition count
        self.write_stats
            .acquisition_count
            .fetch_add(1, Ordering::Relaxed);

        trace!(
            "Write lock acquired (with timeout): {} (wait time: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            wait_time.as_secs_f64() * 1000.0
        );

        Ok(TrackedRwLockWriteGuard {
            guard,
            acquired_at: Instant::now(),
            stats: Arc::clone(&self.write_stats),
            name: self.name.clone(),
        })
    }

    /// Get the read statistics for this lock
    pub fn read_stats(&self) -> LockStats {
        LockStats {
            acquisition_count: self.read_stats.acquisition_count.load(Ordering::Relaxed),
            failed_count: self.read_stats.failed_count.load(Ordering::Relaxed),
            total_wait_time_us: self.read_stats.total_wait_time_us.load(Ordering::Relaxed) as u64,
            total_hold_time_us: self.read_stats.total_hold_time_us.load(Ordering::Relaxed) as u64,
            max_wait_time_us: self.read_stats.max_wait_time_us.load(Ordering::Relaxed) as u64,
            max_hold_time_us: self.read_stats.max_hold_time_us.load(Ordering::Relaxed) as u64,
            deadlock_count: self.read_stats.deadlock_count.load(Ordering::Relaxed),
        }
    }

    /// Get the write statistics for this lock
    pub fn write_stats(&self) -> LockStats {
        LockStats {
            acquisition_count: self.write_stats.acquisition_count.load(Ordering::Relaxed),
            failed_count: self.write_stats.failed_count.load(Ordering::Relaxed),
            total_wait_time_us: self.write_stats.total_wait_time_us.load(Ordering::Relaxed) as u64,
            total_hold_time_us: self.write_stats.total_hold_time_us.load(Ordering::Relaxed) as u64,
            max_wait_time_us: self.write_stats.max_wait_time_us.load(Ordering::Relaxed) as u64,
            max_hold_time_us: self.write_stats.max_hold_time_us.load(Ordering::Relaxed) as u64,
            deadlock_count: self.write_stats.deadlock_count.load(Ordering::Relaxed),
        }
    }

    /// Reset the read statistics
    pub fn reset_read_stats(&self) {
        self.read_stats
            .acquisition_count
            .store(0, Ordering::Relaxed);
        self.read_stats.failed_count.store(0, Ordering::Relaxed);
        self.read_stats
            .total_wait_time_us
            .store(0, Ordering::Relaxed);
        self.read_stats
            .total_hold_time_us
            .store(0, Ordering::Relaxed);
        self.read_stats.max_wait_time_us.store(0, Ordering::Relaxed);
        self.read_stats.max_hold_time_us.store(0, Ordering::Relaxed);
        self.read_stats.deadlock_count.store(0, Ordering::Relaxed);
    }

    /// Reset the write statistics
    pub fn reset_write_stats(&self) {
        self.write_stats
            .acquisition_count
            .store(0, Ordering::Relaxed);
        self.write_stats.failed_count.store(0, Ordering::Relaxed);
        self.write_stats
            .total_wait_time_us
            .store(0, Ordering::Relaxed);
        self.write_stats
            .total_hold_time_us
            .store(0, Ordering::Relaxed);
        self.write_stats
            .max_wait_time_us
            .store(0, Ordering::Relaxed);
        self.write_stats
            .max_hold_time_us
            .store(0, Ordering::Relaxed);
        self.write_stats.deadlock_count.store(0, Ordering::Relaxed);
    }

    /// Reset all statistics
    pub fn reset_stats(&self) {
        self.reset_read_stats();
        self.reset_write_stats();
    }

    /// Get the name of this lock
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Helper method to update max read wait time atomically
    fn update_max_read_wait_time(&self, wait_time_us: usize) {
        let mut current_max = self.read_stats.max_wait_time_us.load(Ordering::Relaxed);

        while wait_time_us > current_max {
            match self.read_stats.max_wait_time_us.compare_exchange(
                current_max,
                wait_time_us,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    /// Helper method to update max write wait time atomically
    fn update_max_write_wait_time(&self, wait_time_us: usize) {
        let mut current_max = self.write_stats.max_wait_time_us.load(Ordering::Relaxed);

        while wait_time_us > current_max {
            match self.write_stats.max_wait_time_us.compare_exchange(
                current_max,
                wait_time_us,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }
}

impl<T> Default for TrackedRwLock<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<'a, T> Drop for TrackedRwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        // Calculate how long the lock was held
        let hold_time = self.acquired_at.elapsed();
        let hold_time_us = hold_time.as_micros() as usize;

        // Update hold time statistics
        self.stats
            .total_hold_time_us
            .fetch_add(hold_time_us, Ordering::Relaxed);

        // Update max hold time atomically
        let mut current_max = self.stats.max_hold_time_us.load(Ordering::Relaxed);

        while hold_time_us > current_max {
            match self.stats.max_hold_time_us.compare_exchange(
                current_max,
                hold_time_us,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }

        trace!(
            "Read lock released: {} (held for: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            hold_time.as_secs_f64() * 1000.0
        );
    }
}

impl<'a, T> std::ops::Deref for TrackedRwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<'a, T> Drop for TrackedRwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        // Calculate how long the lock was held
        let hold_time = self.acquired_at.elapsed();
        let hold_time_us = hold_time.as_micros() as usize;

        // Update hold time statistics
        self.stats
            .total_hold_time_us
            .fetch_add(hold_time_us, Ordering::Relaxed);

        // Update max hold time atomically
        let mut current_max = self.stats.max_hold_time_us.load(Ordering::Relaxed);

        while hold_time_us > current_max {
            match self.stats.max_hold_time_us.compare_exchange(
                current_max,
                hold_time_us,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }

        trace!(
            "Write lock released: {} (held for: {:.2}ms)",
            self.name.as_deref().unwrap_or("unnamed"),
            hold_time.as_secs_f64() * 1000.0
        );
    }
}

impl<'a, T> std::ops::Deref for TrackedRwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<'a, T> std::ops::DerefMut for TrackedRwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_tracked_mutex_basic() {
        let mutex = TrackedMutex::new(0);

        {
            let mut guard = mutex.lock();
            *guard += 1;
        }

        {
            let guard = mutex.lock();
            assert_eq!(*guard, 1);
        }

        let stats = mutex.stats();
        assert_eq!(stats.acquisition_count, 2);
        assert_eq!(stats.failed_count, 0);
    }

    #[test]
    fn test_tracked_mutex_try_lock() {
        let mutex = TrackedMutex::new(0);

        // First lock should succeed
        let guard1 = mutex.try_lock();
        assert!(guard1.is_some());

        // Second lock should fail
        let guard2 = mutex.try_lock();
        assert!(guard2.is_none());

        // Release the first lock
        drop(guard1);

        // Third lock should succeed
        let guard3 = mutex.try_lock();
        assert!(guard3.is_some());

        let stats = mutex.stats();
        assert_eq!(stats.acquisition_count, 2);
        assert_eq!(stats.failed_count, 0); // Failed try_lock doesn't count as a failure
    }

    #[test]
    fn test_tracked_mutex_timeout() {
        let mutex = TrackedMutex::new(0);
        let mutex_arc = Arc::new(mutex.clone());
        let mutex_clone = Arc::clone(&mutex_arc);

        // First lock
        let guard = mutex_arc.lock();

        // Try to lock with timeout (should fail)
        let thread = thread::spawn(move || {
            // Use a very short timeout to ensure it fails quickly
            let result = mutex_clone.try_lock_for(Duration::from_millis(10));
            // Verify the result is a timeout error (should fail since the lock is held)
            if !matches!(result, Err(LockError::Timeout(_))) {
                panic!("Expected timeout error");
            }
        });

        // Release the first lock
        drop(guard);

        // Wait for the thread to complete
        thread.join().unwrap();

        let stats = mutex_arc.stats();
        assert_eq!(stats.acquisition_count, 1);
        assert_eq!(stats.failed_count, 1);
    }

    #[test]
    fn test_tracked_mutex_contention() {
        let mutex = Arc::new(TrackedMutex::with_name(0, "test_mutex"));
        let threads = 10;
        let iterations = 100;

        let mut handles = vec![];

        for _ in 0..threads {
            let mutex = Arc::clone(&mutex);
            let handle = thread::spawn(move || {
                for _ in 0..iterations {
                    let mut guard = mutex.lock();
                    *guard += 1;
                    // Small sleep to induce contention
                    thread::sleep(Duration::from_micros(10));
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Final value should be threads * iterations
        let guard = mutex.lock();
        assert_eq!(*guard, threads * iterations);

        // Check stats
        let stats = mutex.stats();
        assert_eq!(stats.acquisition_count, threads * iterations + 1); // +1 for the final check

        // Should have some wait time due to contention
        assert!(stats.total_wait_time_us > 0);
    }

    #[test]
    fn test_tracked_rwlock_basic() {
        let rwlock = TrackedRwLock::new(0);

        // Acquire read lock
        {
            let guard = rwlock.read();
            assert_eq!(*guard, 0);
        }

        // Acquire write lock
        {
            let mut guard = rwlock.write();
            *guard += 1;
        }

        // Acquire read lock again
        {
            let guard = rwlock.read();
            assert_eq!(*guard, 1);
        }

        // Check stats
        let read_stats = rwlock.read_stats();
        let write_stats = rwlock.write_stats();

        assert_eq!(read_stats.acquisition_count, 2);
        assert_eq!(write_stats.acquisition_count, 1);
    }

    #[test]
    fn test_tracked_rwlock_multiple_readers() {
        let rwlock = Arc::new(TrackedRwLock::new(0));

        // Start some readers
        let mut handles = vec![];

        for _ in 0..5 {
            let rwlock = Arc::clone(&rwlock);
            let handle = thread::spawn(move || {
                let guard = rwlock.read();
                thread::sleep(Duration::from_millis(50));
                *guard
            });

            handles.push(handle);
        }

        // All readers should succeed
        for handle in handles {
            let _ = handle.join().unwrap();
        }

        // Check stats
        let read_stats = rwlock.read_stats();
        assert_eq!(read_stats.acquisition_count, 5);
    }

    #[test]
    fn test_tracked_rwlock_reader_writer_contention() {
        let rwlock = Arc::new(TrackedRwLock::new(0));

        // Acquire read lock
        let read_lock = rwlock.read();

        // Try to acquire write lock (should fail with timeout)
        let rwlock_clone = Arc::clone(&rwlock);
        let writer = thread::spawn(move || {
            let result = rwlock_clone.try_write_for(Duration::from_millis(100));
            assert!(matches!(result, Err(LockError::Timeout(_))));
        });

        writer.join().unwrap();

        // Release read lock
        drop(read_lock);

        // Check stats
        let write_stats = rwlock.write_stats();
        assert_eq!(write_stats.failed_count, 1);
    }

    #[test]
    fn test_tracked_rwlock_writer_blocks_readers() {
        let rwlock = Arc::new(TrackedRwLock::new(0));

        // Acquire write lock
        let write_lock = rwlock.write();

        // Try to acquire read lock (should fail with timeout)
        let rwlock_clone = Arc::clone(&rwlock);
        let reader = thread::spawn(move || {
            let result = rwlock_clone.try_read_for(Duration::from_millis(100));
            assert!(matches!(result, Err(LockError::Timeout(_))));
        });

        reader.join().unwrap();

        // Release write lock
        drop(write_lock);

        // Check stats
        let read_stats = rwlock.read_stats();
        assert_eq!(read_stats.failed_count, 1);
    }
}
