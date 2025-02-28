//! Shutdown Manager for Lion Runtime
//!
//! Handles graceful shutdown of the system with two-phase process.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use parking_lot::Mutex;
use thiserror::Error;
use tokio::sync::{broadcast, Semaphore};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::config::RuntimeConfig;

/// Errors that can occur during shutdown
#[derive(Debug, Error)]
pub enum ShutdownError {
    #[error("Shutdown timeout")]
    Timeout,

    #[error("Component shutdown failed: {0}")]
    ComponentFailed(String),

    #[error("Shutdown already in progress")]
    AlreadyInProgress,
}

/// Shutdown handle for a component
#[derive(Clone)]
pub struct ShutdownHandle {
    /// Shutdown signal receiver
    receiver: Arc<Mutex<broadcast::Receiver<()>>>,

    /// Completion semaphore
    completion: Arc<Semaphore>,

    /// ID of this handle
    id: String,
}

impl ShutdownHandle {
    /// Create a new shutdown handle
    fn new(receiver: broadcast::Receiver<()>, completion: Arc<Semaphore>, id: String) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            completion,
            id,
        }
    }

    /// Wait for shutdown signal
    pub async fn wait_for_shutdown(&mut self) -> Result<()> {
        // Get our own copy of the receiver before awaiting
        // This is necessary because we can't hold the mutex lock across an await point
        // as it would make the future not Send
        let mut local_rx = {
            // Create a new receiver by cloning from the existing one
            // The lock is dropped at the end of this block
            let guard = self.receiver.lock();
            guard.resubscribe()
        };

        // Now await on the local receiver without holding the lock
        local_rx
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!("Shutdown signal error: {}", e))
    }

    /// Signal that shutdown is complete
    pub fn shutdown_complete(&self) {
        self.completion.add_permits(1);
    }

    /// Get the ID of this handle
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Manager for system shutdown
pub struct ShutdownManager {
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,

    /// Completion semaphore
    completion: Arc<Semaphore>,

    /// Number of registered components
    component_count: Mutex<usize>,

    /// Shutdown timeout in seconds
    timeout_seconds: u32,

    /// Whether shutdown is in progress
    in_progress: Mutex<bool>,

    /// Registered components
    components: Mutex<HashMap<String, String>>,
}

impl ShutdownManager {
    /// Create a new shutdown manager
    pub fn new(config: RuntimeConfig) -> Self {
        let (tx, _) = broadcast::channel(16);

        Self {
            shutdown_tx: tx,
            completion: Arc::new(Semaphore::new(0)),
            component_count: Mutex::new(0),
            timeout_seconds: config.shutdown_timeout,
            in_progress: Mutex::new(false),
            components: Mutex::new(HashMap::new()),
        }
    }

    /// Register a component for shutdown
    pub fn register_component(&self, name: &str) -> ShutdownHandle {
        let mut count = self.component_count.lock();
        *count += 1;

        let id = Uuid::new_v4().to_string();

        // Add to component map
        self.components.lock().insert(id.clone(), name.to_string());

        info!("Registered component for shutdown: {} ({})", name, id);

        ShutdownHandle::new(self.shutdown_tx.subscribe(), self.completion.clone(), id)
    }

    /// Request a graceful shutdown
    pub async fn request_shutdown(&self) -> Result<()> {
        info!("Initiating graceful shutdown");

        // Check if shutdown is already in progress
        {
            let mut in_progress = self.in_progress.lock();
            if *in_progress {
                return Err(ShutdownError::AlreadyInProgress.into());
            }

            // Mark shutdown as in progress
            *in_progress = true;
        }

        // Get component count
        let component_count = *self.component_count.lock();

        // Phase A: Signal shutdown to all components
        info!("Phase A: Signaling all components to stop accepting new work");

        // Only try to send if there are actually components registered
        if component_count > 0 {
            self.shutdown_tx.send(()).map_err(|_| {
                ShutdownError::ComponentFailed("Failed to send shutdown signal".to_string())
            })?;
        } else {
            info!("No components registered, skipping shutdown signal");
        }

        // Phase B: Wait for components to complete (with timeout)
        info!("Phase B: Waiting for components to complete");
        let shutdown_timeout = Duration::from_secs(self.timeout_seconds as u64);

        match timeout(shutdown_timeout, self.wait_for_completion(component_count)).await {
            Ok(result) => result?,
            Err(_) => {
                warn!("Shutdown timeout after {} seconds", self.timeout_seconds);
                self.log_incomplete_components();
                return Err(ShutdownError::Timeout.into());
            }
        }

        info!("All components shut down successfully");

        Ok(())
    }

    /// Wait for all components to complete
    async fn wait_for_completion(&self, count: usize) -> Result<()> {
        // If count is 0, return immediately
        if count == 0 {
            return Ok(());
        }

        // Acquire the semaphore permits (one per component)
        match self.completion.acquire_many(count as u32).await {
            Ok(permits) => {
                // Immediately release the permits
                permits.forget();
                Ok(())
            }
            Err(e) => Err(ShutdownError::ComponentFailed(format!("Semaphore error: {}", e)).into()),
        }
    }

    /// Log components that haven't completed shutdown
    fn log_incomplete_components(&self) {
        let completed_count = self.completion.available_permits();
        let total_count = *self.component_count.lock();
        let incomplete_count = total_count - completed_count;

        error!(
            "{} of {} components did not complete shutdown",
            incomplete_count, total_count
        );

        // Log which components are still running
        // In a real implementation, we would have a way to track which components
        // have completed shutdown and which haven't
        let components = self.components.lock();
        info!("Components registered for shutdown: {}", components.len());

        for (id, name) in components.iter() {
            debug!("  Component: {} ({})", name, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_shutdown_manager() {
        // Create a config with a short timeout
        let mut config = RuntimeConfig::default();
        config.shutdown_timeout = 2; // Use a short timeout for test

        // Create the shutdown manager
        let manager = Arc::new(ShutdownManager::new(config));

        // Register some components
        let handle1 = manager.register_component("Component1");
        let handle2 = manager.register_component("Component2");
        let handle3 = manager.register_component("Component3");

        // Make sure all handles call shutdown_complete right away
        // to avoid race conditions in the test
        handle1.shutdown_complete();
        handle2.shutdown_complete();
        handle3.shutdown_complete();

        // Short delay to ensure all permits are registered
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Request shutdown - this should succeed since all components
        // have already completed their shutdown
        manager
            .request_shutdown()
            .await
            .expect("Shutdown should succeed");
    }

    #[tokio::test]
    async fn test_shutdown_timeout() {
        // This test creates a component that never completes shutdown
        // and verifies that the shutdown manager times out

        // Use a very short timeout to make the test run quickly
        let config = RuntimeConfig {
            shutdown_timeout: 1, // 1 second timeout
            ..RuntimeConfig::default()
        };

        // Create the shutdown manager
        let manager = Arc::new(ShutdownManager::new(config));

        // Register components
        let never_completes = manager.register_component("SlowComponent");

        // Start a task that will wait for shutdown but never complete
        let _slow_task = tokio::spawn({
            let mut handle = never_completes;
            async move {
                // Wait for shutdown signal
                if let Err(e) = handle.wait_for_shutdown().await {
                    error!("SlowComponent error waiting for shutdown: {}", e);
                    return;
                }
                info!("SlowComponent: received shutdown signal but will never complete");

                // Simulate a component that hangs and never completes
                sleep(Duration::from_secs(10)).await;

                // We should never reach this point as the test will complete before this
                handle.shutdown_complete();
            }
        });

        // Allow the slow_task to start waiting for shutdown
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Request shutdown (should timeout)
        info!("Starting shutdown timeout test...");
        let result = manager.request_shutdown().await;

        // Verify that we got a timeout error
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Shutdown timeout"),
            "Expected Timeout error, got: {}",
            err
        );
    }
}
