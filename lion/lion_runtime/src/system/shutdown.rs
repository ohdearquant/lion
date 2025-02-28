//! Shutdown Manager for Lion Runtime
//!
//! Handles graceful shutdown of the system with two-phase process.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;
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
        self.receiver
            .lock()
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!("Shutdown signal error: {}", e))
    }

    /// Signal that shutdown is complete
    pub fn shutdown_complete(&self) {
        let _ = self.completion.add_permits(1);
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
        let mut in_progress = self.in_progress.lock();
        if *in_progress {
            return Err(ShutdownError::AlreadyInProgress.into());
        }

        // Mark shutdown as in progress
        *in_progress = true;

        // Get component count
        let component_count = *self.component_count.lock();

        // Phase A: Signal shutdown to all components
        info!("Phase A: Signaling all components to stop accepting new work");
        self.shutdown_tx.send(()).map_err(|_| {
            ShutdownError::ComponentFailed("Failed to send shutdown signal".to_string())
        })?;

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
        let mut completed_count = self.completion.available_permits() as usize;
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
        config.shutdown_timeout = 5;

        // Create the shutdown manager
        let manager = Arc::new(ShutdownManager::new(config));

        // Register some components
        let handle1 = manager.register_component("Component1");
        let handle2 = manager.register_component("Component2");
        let handle3 = manager.register_component("Component3");

        // Start a task that will complete shutdown for component 1
        let manager_clone = manager.clone();
        let mut handle1_clone = handle1.clone();
        tokio::spawn(async move {
            handle1_clone.wait_for_shutdown().await.unwrap();
            info!("Component1 shutting down");
            sleep(Duration::from_millis(100)).await;
            handle1_clone.shutdown_complete();
        });

        // Start a task that will complete shutdown for component 2
        let manager_clone = manager.clone();
        let mut handle2_clone = handle2.clone();
        tokio::spawn(async move {
            handle2_clone.wait_for_shutdown().await.unwrap();
            info!("Component2 shutting down");
            sleep(Duration::from_millis(200)).await;
            handle2_clone.shutdown_complete();
        });

        // Start a task that will complete shutdown for component 3
        let manager_clone = manager.clone();
        let mut handle3_clone = handle3.clone();
        tokio::spawn(async move {
            handle3_clone.wait_for_shutdown().await.unwrap();
            info!("Component3 shutting down");
            sleep(Duration::from_millis(300)).await;
            handle3_clone.shutdown_complete();
        });

        // Request shutdown
        manager.request_shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_shutdown_timeout() {
        // Create a config with a very short timeout
        let mut config = RuntimeConfig::default();
        config.shutdown_timeout = 1;

        // Create the shutdown manager
        let manager = Arc::new(ShutdownManager::new(config));

        // Register a component that won't shut down
        let handle = manager.register_component("SlowComponent");

        // Start a task that will wait for shutdown but never complete
        let manager_clone = manager.clone();
        let mut handle_clone = handle.clone();
        tokio::spawn(async move {
            handle_clone.wait_for_shutdown().await.unwrap();
            info!("SlowComponent got shutdown signal but will never complete");
            sleep(Duration::from_secs(10)).await;
            // Never call shutdown_complete()
        });

        // Request shutdown (should timeout)
        let result = manager.request_shutdown().await;
        assert!(result.is_err());
        match result {
            Err(e) => {
                let e = e.downcast::<ShutdownError>();
                assert!(e.is_ok());
                assert!(matches!(e.unwrap(), ShutdownError::Timeout));
            }
            _ => panic!("Expected an error"),
        }
    }
}
