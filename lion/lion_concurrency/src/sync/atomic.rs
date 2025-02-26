//! Atomic operations and data structures.
//!
//! Provides efficient, lock-free synchronization primitives.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// A counter that can be incremented and read atomically.
#[derive(Debug)]
pub struct AtomicCounter {
    /// The current value of the counter
    value: AtomicUsize,

    /// The starting value of the counter
    initial_value: usize,
}

impl AtomicCounter {
    /// Create a new atomic counter with an initial value.
    pub fn new(initial_value: usize) -> Self {
        Self {
            value: AtomicUsize::new(initial_value),
            initial_value,
        }
    }

    /// Increment the counter and return the new value.
    pub fn increment(&self) -> usize {
        self.value.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement the counter and return the new value.
    pub fn decrement(&self) -> usize {
        self.value.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Get the current value of the counter.
    pub fn get(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }

    /// Reset the counter to its initial value.
    pub fn reset(&self) -> usize {
        self.value.swap(self.initial_value, Ordering::SeqCst)
    }

    /// Set the counter to a new value and return the old value.
    pub fn set(&self, new_value: usize) -> usize {
        self.value.swap(new_value, Ordering::SeqCst)
    }
}

impl Default for AtomicCounter {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A flag that can be atomically set and unset.
#[derive(Debug)]
pub struct AtomicFlag {
    /// The flag value
    flag: AtomicBool,
}

impl AtomicFlag {
    /// Create a new atomic flag with the specified initial state.
    pub fn new(initial_state: bool) -> Self {
        Self {
            flag: AtomicBool::new(initial_state),
        }
    }

    /// Set the flag to true and return the previous value.
    pub fn set(&self) -> bool {
        self.flag.swap(true, Ordering::SeqCst)
    }

    /// Set the flag to false and return the previous value.
    pub fn unset(&self) -> bool {
        self.flag.swap(false, Ordering::SeqCst)
    }

    /// Get the current state of the flag.
    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Toggle the flag and return the new value.
    pub fn toggle(&self) -> bool {
        !self.flag.fetch_xor(true, Ordering::SeqCst)
    }

    /// Set the flag if it's not already set.
    ///
    /// Returns true if the flag was set by this call, false if it was already set.
    pub fn try_set(&self) -> bool {
        !self.flag.swap(true, Ordering::SeqCst) && true
    }

    /// Wait for the flag to be set, returning true if it's set within the timeout.
    pub fn wait_for_set(&self, timeout: Duration) -> bool {
        let start = Instant::now();

        while !self.is_set() {
            if start.elapsed() >= timeout {
                return false;
            }

            // Short sleep to avoid spinning
            std::thread::sleep(Duration::from_micros(10));
        }

        true
    }

    /// Wait for the flag to be unset, returning true if it's unset within the timeout.
    pub fn wait_for_unset(&self, timeout: Duration) -> bool {
        let start = Instant::now();

        while self.is_set() {
            if start.elapsed() >= timeout {
                return false;
            }

            // Short sleep to avoid spinning
            std::thread::sleep(Duration::from_micros(10));
        }

        true
    }
}

impl Default for AtomicFlag {
    fn default() -> Self {
        Self::new(false)
    }
}

/// A sequence number that can be safely incremented across threads.
#[derive(Debug)]
pub struct AtomicSequence {
    /// The current sequence number
    value: AtomicU64,
}

impl AtomicSequence {
    /// Create a new atomic sequence starting from the specified value.
    pub fn new(start: u64) -> Self {
        Self {
            value: AtomicU64::new(start),
        }
    }

    /// Get the next sequence number.
    pub fn next(&self) -> u64 {
        self.value.fetch_add(1, Ordering::SeqCst)
    }

    /// Get the current sequence number without incrementing.
    pub fn current(&self) -> u64 {
        self.value.load(Ordering::SeqCst)
    }

    /// Reset the sequence to a specific value and return the old value.
    pub fn reset(&self, new_value: u64) -> u64 {
        self.value.swap(new_value, Ordering::SeqCst)
    }
}

impl Default for AtomicSequence {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A version counter for tracking changes to data.
///
/// This helps detect concurrent modifications by allowing code to check if a value
/// has been changed by another thread.
#[derive(Debug)]
pub struct VersionCounter {
    /// The current version number
    version: AtomicU64,
}

impl VersionCounter {
    /// Create a new version counter starting at 1.
    pub fn new() -> Self {
        Self {
            version: AtomicU64::new(1),
        }
    }

    /// Increment the version number and return the new value.
    pub fn increment(&self) -> u64 {
        self.version.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Get the current version number.
    pub fn get(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    /// Check if the version has changed since the specified version.
    pub fn has_changed_since(&self, version: u64) -> bool {
        self.get() > version
    }

    /// Check and update the version if it matches.
    ///
    /// If the current version is equal to `expected_version`, increments the version
    /// and returns true. Otherwise, returns false.
    pub fn check_and_update(&self, expected_version: u64) -> bool {
        let result = self.version.compare_exchange(
            expected_version,
            expected_version + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );

        result.is_ok()
    }
}

impl Default for VersionCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_atomic_counter() {
        let counter = AtomicCounter::new(5);

        assert_eq!(counter.get(), 5);
        assert_eq!(counter.increment(), 6);
        assert_eq!(counter.increment(), 7);
        assert_eq!(counter.decrement(), 6);
        assert_eq!(counter.reset(), 6); // Returns old value
        assert_eq!(counter.get(), 5); // Reset to initial value
    }

    #[test]
    fn test_atomic_counter_threads() {
        let counter = Arc::new(AtomicCounter::new(0));
        let threads = 10;
        let increments_per_thread = 1000;

        let mut handles = vec![];

        for _ in 0..threads {
            let counter = Arc::clone(&counter);
            let handle = thread::spawn(move || {
                for _ in 0..increments_per_thread {
                    counter.increment();
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.get(), threads * increments_per_thread);
    }

    #[test]
    fn test_atomic_flag() {
        let flag = AtomicFlag::new(false);

        assert_eq!(flag.is_set(), false);
        assert_eq!(flag.set(), false); // Returns old value
        assert_eq!(flag.is_set(), true);
        assert_eq!(flag.toggle(), false); // Returns new value
        assert_eq!(flag.is_set(), false);
        assert_eq!(flag.toggle(), true); // Returns new value
        assert_eq!(flag.is_set(), true);
        assert_eq!(flag.unset(), true); // Returns old value
        assert_eq!(flag.is_set(), false);
    }

    #[test]
    fn test_atomic_flag_threads() {
        let flag = Arc::new(AtomicFlag::new(false));
        let thread_count = 5;

        // Spawn threads that try to set the flag
        let mut handles = vec![];
        let success_count = Arc::new(AtomicUsize::new(0));

        for _ in 0..thread_count {
            let flag = Arc::clone(&flag);
            let success_count = Arc::clone(&success_count);

            let handle = thread::spawn(move || {
                if flag.try_set() {
                    success_count.fetch_add(1, Ordering::SeqCst);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Only one thread should have successfully set the flag
        assert_eq!(success_count.load(Ordering::SeqCst), 1);
        assert!(flag.is_set());
    }

    #[test]
    fn test_atomic_flag_wait() {
        let flag = Arc::new(AtomicFlag::new(false));
        let flag_clone = Arc::clone(&flag);

        // Spawn a thread that sets the flag after a delay
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            flag_clone.set();
        });

        // Wait for the flag to be set
        let result = flag.wait_for_set(Duration::from_millis(100));
        assert!(result);

        handle.join().unwrap();
    }

    #[test]
    fn test_atomic_sequence() {
        let seq = AtomicSequence::new(100);

        assert_eq!(seq.current(), 100);
        assert_eq!(seq.next(), 100); // Returns old value
        assert_eq!(seq.current(), 101);
        assert_eq!(seq.next(), 101); // Returns old value
        assert_eq!(seq.current(), 102);
        assert_eq!(seq.reset(50), 102); // Returns old value
        assert_eq!(seq.current(), 50);
    }

    #[test]
    fn test_version_counter() {
        let version = VersionCounter::new();

        assert_eq!(version.get(), 1);
        assert_eq!(version.increment(), 2);
        assert!(version.has_changed_since(1));
        assert!(!version.has_changed_since(2));

        // Should succeed because current version is 2
        assert!(version.check_and_update(2));
        assert_eq!(version.get(), 3);

        // Should fail because current version is now 3
        assert!(!version.check_and_update(2));
    }

    #[test]
    fn test_version_counter_threads() {
        let version = Arc::new(VersionCounter::new());
        let success_count = Arc::new(AtomicUsize::new(0));

        // Spawn threads that try to update the version if it's still 1
        let thread_count = 5;
        let mut handles = vec![];

        for _ in 0..thread_count {
            let version = Arc::clone(&version);
            let success_count = Arc::clone(&success_count);

            let handle = thread::spawn(move || {
                if version.check_and_update(1) {
                    success_count.fetch_add(1, Ordering::SeqCst);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Only one thread should have successfully updated from version 1
        assert_eq!(success_count.load(Ordering::SeqCst), 1);
        assert_eq!(version.get(), 2);
    }
}
