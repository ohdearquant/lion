//! Actor supervision and failure recovery.
//!
//! Supervisors monitor actors and handle failures according to supervision
//! strategies, supporting the actor system's fault tolerance.

use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use super::system::ActorSystem;

/// Error type for supervisor operations
#[derive(Error, Debug)]
pub enum SupervisionError {
    /// Actor reference is not valid or not found
    #[error("actor not found: {0}")]
    ActorNotFound(String),

    /// Max restart attempts exceeded
    #[error("max restart attempts exceeded for actor: {0}")]
    MaxRestartExceeded(String),

    /// Restart timed out
    #[error("restart timed out for actor: {0}")]
    RestartTimeout(String),
}

/// Strategy for handling actor failures
#[derive(Debug, Clone, PartialEq)]
pub enum SupervisionStrategy {
    /// Restart just the failed actor
    RestartOne,

    /// Restart the failed actor and all its children
    RestartSubtree,

    /// Escalate the failure to the parent supervisor
    Escalate,

    /// Stop the failed actor
    Stop,
}

/// Configuration for a supervisor
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Strategy to use when an actor fails
    pub strategy: SupervisionStrategy,

    /// Maximum number of restart attempts within the interval
    pub max_restarts: usize,

    /// Time interval for counting restarts
    pub restart_window: Duration,

    /// Delay before restarting
    pub restart_delay: Duration,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            strategy: SupervisionStrategy::RestartOne,
            max_restarts: 10,
            restart_window: Duration::from_secs(60),
            restart_delay: Duration::from_millis(100),
        }
    }
}

/// Interface for supervising actors
pub trait Supervisor: Send + Sync {
    /// Handle a failed actor
    fn handle_failure(&self, actor_id: &str, error: &dyn std::error::Error) -> SupervisionStrategy;

    /// Get the supervisor configuration
    fn config(&self) -> &SupervisorConfig;
}

/// A basic implementation of the Supervisor trait
#[allow(dead_code)]
pub struct BasicSupervisor {
    /// Configuration for this supervisor
    config: SupervisorConfig,

    /// Reference to the actor system
    system: Arc<ActorSystem>,
}

impl BasicSupervisor {
    /// Create a new basic supervisor
    pub fn new(system: Arc<ActorSystem>, config: SupervisorConfig) -> Self {
        Self { config, system }
    }

    /// Restart an actor with the given ID
    pub fn restart_actor(&self, actor_id: &str) -> Result<(), SupervisionError> {
        info!("Restarting actor: {}", actor_id);

        // In a real implementation, we would:
        // 1. Look up the actor in the actor system
        // 2. Implement restart logic based on actor type
        // 3. Track restart attempts and enforce max_restarts

        // For now, just simulate a restart
        std::thread::sleep(self.config.restart_delay);

        debug!("Actor restarted: {}", actor_id);
        Ok(())
    }
}

impl Supervisor for BasicSupervisor {
    fn handle_failure(&self, actor_id: &str, error: &dyn std::error::Error) -> SupervisionStrategy {
        error!("Actor failure: {} - Error: {}", actor_id, error);

        match &self.config.strategy {
            SupervisionStrategy::RestartOne => {
                if let Err(e) = self.restart_actor(actor_id) {
                    warn!("Failed to restart actor {}: {}", actor_id, e);
                    SupervisionStrategy::Stop
                } else {
                    SupervisionStrategy::RestartOne
                }
            }
            strategy => strategy.clone(),
        }
    }

    fn config(&self) -> &SupervisorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "TestError: {}", self.0)
        }
    }

    impl Error for TestError {}

    #[test]
    fn test_basic_supervisor_strategy() {
        let system = Arc::new(ActorSystem::new());
        let config = SupervisorConfig {
            strategy: SupervisionStrategy::RestartOne,
            ..Default::default()
        };

        let supervisor = BasicSupervisor::new(system, config);
        let error = TestError("test failure");

        let strategy = supervisor.handle_failure("test-actor", &error);
        assert_eq!(strategy, SupervisionStrategy::RestartOne);
    }

    #[test]
    fn test_supervisor_config_default() {
        let config = SupervisorConfig::default();

        assert_eq!(config.strategy, SupervisionStrategy::RestartOne);
        assert_eq!(config.max_restarts, 10);
        assert_eq!(config.restart_window, Duration::from_secs(60));
        assert_eq!(config.restart_delay, Duration::from_millis(100));
    }
}
