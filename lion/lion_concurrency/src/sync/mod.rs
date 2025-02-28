//! Synchronization primitives optimized for the Lion architecture.
//!
//! This module provides efficient synchronization primitives for concurrent
//! programming in the Lion microkernel:
//!
//! - Atomic operations and data structures for lock-free synchronization
//! - Enhanced locks with timeouts, statistics, and diagnostics capabilities

pub mod atomic;
pub mod lock;

// Re-export key types from atomic
pub use atomic::{AtomicCounter, AtomicFlag, AtomicSequence, VersionCounter};

// Re-export key types from lock
pub use lock::{
    LockError, LockStats, TrackedMutex, TrackedMutexGuard, TrackedRwLock, TrackedRwLockReadGuard,
    TrackedRwLockWriteGuard,
};

#[cfg(test)]
mod tests {
    // Integration tests for the sync module can be added here
}
