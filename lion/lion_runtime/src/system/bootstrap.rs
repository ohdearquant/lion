//! System Bootstrap for Lion Runtime
//!
//! Handles system initialization and bootstrap sequence.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use thiserror::Error;
use tokio::sync::OnceCell;
use tokio::time::timeout;
use tracing::{error, info, warn};

use super::config::RuntimeConfig;
use super::shutdown::ShutdownManager;

/// Errors that can occur during bootstrap
#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Phase {0} bootstrap failed: {1}")]
    PhaseFailed(u8, String),

    #[error("Bootstrap timeout in phase {0}")]
    Timeout(u8),

    #[error("Dependency {0} not initialized")]
    DependencyNotInitialized(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Bootstrap phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapPhase {
    /// Phase 0: Core system setup
    Core = 0,

    /// Phase 1: Essential services
    Essential = 1,

    /// Phase 2: Standard services
    Standard = 2,

    /// Phase 3: Optional services
    Optional = 3,

    /// System is fully bootstrapped
    Complete = 4,
}

/// State of the system
pub struct System {
    /// System configuration
    config: RuntimeConfig,

    /// Current bootstrap phase
    _phase: BootstrapPhase,

    /// Shutdown manager (wrapped in Arc and OnceCell for initialization)
    shutdown_manager: OnceCell<Arc<ShutdownManager>>,
}

impl System {
    /// Create a new system
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        Ok(Self {
            config,
            _phase: BootstrapPhase::Core,
            shutdown_manager: OnceCell::new(),
        })
    }

    /// Bootstrap the system
    pub async fn bootstrap(&self) -> Result<()> {
        info!("Starting system bootstrap");

        // Phase 0: Core system setup
        self.bootstrap_phase_0().await?;

        // Phase 1: Essential services
        self.bootstrap_phase_1().await?;

        // Phase 2: Standard services
        self.bootstrap_phase_2().await?;

        // Phase 3: Optional services
        self.bootstrap_phase_3().await?;

        info!("System bootstrap complete");

        Ok(())
    }

    /// Phase 0: Core system setup (memory, logging, minimal config)
    async fn bootstrap_phase_0(&self) -> Result<()> {
        info!("Bootstrap Phase 0: Core system setup");

        let phase_timeout = self.config.bootstrap_timeouts.get(&0).unwrap_or(&30);

        match timeout(Duration::from_secs(*phase_timeout as u64), async {
            // Initialize shutdown manager
            if self.shutdown_manager.get().is_none() {
                let manager = ShutdownManager::new(self.config.clone());
                let _ = self.shutdown_manager.set(Arc::new(manager));
            }

            // Basic validations
            if self.config.plugin_directory.is_empty() {
                return Err(BootstrapError::ConfigError(
                    "Plugin directory not configured".to_string(),
                )
                .into());
            }

            Ok(())
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(BootstrapError::Timeout(0).into()),
        }
    }

    /// Phase 1: Essential services
    async fn bootstrap_phase_1(&self) -> Result<()> {
        info!("Bootstrap Phase 1: Essential services");

        let phase_timeout = self.config.bootstrap_timeouts.get(&1).unwrap_or(&30);

        match timeout(Duration::from_secs(*phase_timeout as u64), async {
            // Initialize necessary directories
            let plugin_dir = std::path::Path::new(&self.config.plugin_directory);
            if !plugin_dir.exists() {
                std::fs::create_dir_all(plugin_dir).context(format!(
                    "Failed to create plugin directory: {:?}",
                    plugin_dir
                ))?;
            }

            Ok(())
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(BootstrapError::Timeout(1).into()),
        }
    }

    /// Phase 2: Standard services
    async fn bootstrap_phase_2(&self) -> Result<()> {
        info!("Bootstrap Phase 2: Standard services");

        let phase_timeout = self.config.bootstrap_timeouts.get(&2).unwrap_or(&30);

        match timeout(Duration::from_secs(*phase_timeout as u64), async {
            // Initialize any standard services

            Ok(())
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(BootstrapError::Timeout(2).into()),
        }
    }

    /// Phase 3: Optional services
    async fn bootstrap_phase_3(&self) -> Result<()> {
        info!("Bootstrap Phase 3: Optional services");

        let phase_timeout = self.config.bootstrap_timeouts.get(&3).unwrap_or(&30);

        match timeout(Duration::from_secs(*phase_timeout as u64), async {
            // Initialize monitoring service if enabled
            if self.config.monitoring.enabled {
                info!("Initializing monitoring service");
                // In a real implementation, this would initialize the monitoring service
            }

            Ok(())
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(BootstrapError::Timeout(3).into()),
        }
    }

    /// Shut down the system
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down system");

        if let Some(manager) = self.shutdown_manager.get() {
            manager.request_shutdown().await?;
            // Wait a small amount of time to ensure any shutdown tasks have a chance to run
            // This helps prevent race conditions in tests
            tokio::time::sleep(Duration::from_millis(50)).await;
        } else {
            warn!("Shutdown manager not initialized, performing simple shutdown");
        }

        info!("System shutdown complete");

        Ok(())
    }

    /// Get the shutdown manager
    pub fn get_shutdown_manager(&self) -> Result<Arc<ShutdownManager>> {
        self.shutdown_manager.get().cloned().ok_or_else(|| {
            BootstrapError::DependencyNotInitialized("ShutdownManager".to_string()).into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_bootstrap() -> Result<()> {
        // Create a temporary directory for plugins
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();

        // Create a config
        let mut config = RuntimeConfig::default();
        config.plugin_directory = temp_path;
        config.shutdown_timeout = 2; // Set a short timeout for tests

        // Create the system
        let system = System::new(config).unwrap();

        // Bootstrap
        system.bootstrap().await?;

        // Register a component for proper shutdown testing
        let shutdown_manager = system.get_shutdown_manager()?;
        let handle = shutdown_manager.register_component("TestComponent");

        // Set up a component that will cleanly shut down
        tokio::task::spawn({
            let mut handle = handle.clone();
            async move {
                handle.shutdown_complete();
            }
        });

        // Shutdown (now should succeed since we registered a component)
        system.shutdown().await
    }
}
