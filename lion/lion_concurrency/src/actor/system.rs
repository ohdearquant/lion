//! Actor system for managing actor lifecycles and interactions.
//!
//! The ActorSystem provides the infrastructure for actor creation,
//! message routing, and lifecycle management.

use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::supervisor::{BasicSupervisor, Supervisor, SupervisorConfig};
use crate::pool::thread::ThreadPool;

/// Unique identifier for an actor
pub type ActorId = String;

/// Status of an actor
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActorStatus {
    /// Actor is starting up
    Starting,
    /// Actor is running normally
    Running,
    /// Actor is stopping
    Stopping,
    /// Actor has stopped normally
    Stopped,
    /// Actor has crashed
    Crashed,
}

/// Configuration for the actor system
#[derive(Debug, Clone)]
pub struct ActorSystemConfig {
    /// Number of worker threads in the default thread pool
    pub worker_threads: usize,
    /// Default supervision strategy
    pub supervisor_config: SupervisorConfig,
}

impl Default for ActorSystemConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            supervisor_config: SupervisorConfig::default(),
        }
    }
}

/// Metadata about an actor
#[allow(dead_code)]
#[derive(Debug)]
struct ActorMetadata {
    /// Actor's current status
    status: ActorStatus,

    /// Type name of the actor
    actor_type: String,

    /// Actor's supervisor ID
    supervisor_id: Option<ActorId>,
}

/// The central actor system that manages actors
pub struct ActorSystem {
    /// Registry of all actors and their metadata
    registry: Mutex<HashMap<ActorId, ActorMetadata>>,
    /// Default thread pool for executing actor tasks
    thread_pool: ThreadPool,
    /// Configuration for this actor system
    config: ActorSystemConfig,
}

impl ActorSystem {
    /// Create a new actor system with default configuration
    pub fn new() -> Self {
        Self::with_config(ActorSystemConfig::default())
    }

    /// Create a new actor system with the specified configuration
    pub fn with_config(config: ActorSystemConfig) -> Self {
        let thread_pool = ThreadPool::new(config.worker_threads);

        info!(
            "Creating actor system with {} worker threads",
            config.worker_threads
        );

        Self {
            registry: Mutex::new(HashMap::new()),
            thread_pool,
            config,
        }
    }

    /// Register an actor with the system
    pub fn register_actor(&self, id: ActorId, actor_type: &str, supervisor_id: Option<ActorId>) {
        let mut registry = self.registry.lock().unwrap();

        let metadata = ActorMetadata {
            status: ActorStatus::Starting,
            actor_type: actor_type.to_string(),
            supervisor_id,
        };

        registry.insert(id.clone(), metadata);
        debug!("Registered actor: {} (type: {})", id, actor_type);
    }

    /// Update an actor's status
    pub fn update_status(&self, id: &ActorId, status: ActorStatus) {
        let mut registry = self.registry.lock().unwrap();

        if let Some(metadata) = registry.get_mut(id) {
            let old_status = metadata.status;
            metadata.status = status;
            debug!(
                "Actor {} status change: {:?} -> {:?}",
                id, old_status, status
            );
        } else {
            warn!("Attempted to update status for unknown actor: {}", id);
        }
    }

    /// Get the status of an actor
    pub fn get_status(&self, id: &ActorId) -> Option<ActorStatus> {
        let registry = self.registry.lock().unwrap();
        registry.get(id).map(|metadata| metadata.status)
    }

    /// Get the supervisor for an actor
    pub fn get_supervisor_id(&self, id: &ActorId) -> Option<ActorId> {
        let registry = self.registry.lock().unwrap();
        registry
            .get(id)
            .and_then(|metadata| metadata.supervisor_id.clone())
    }

    /// Submit a task to be executed by the actor system's thread pool
    pub fn submit<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let _ = self.thread_pool.execute(f);
    }

    /// Create a default supervisor for this actor system
    pub fn create_default_supervisor(&self) -> Arc<dyn Supervisor> {
        Arc::new(BasicSupervisor::new(
            Arc::new(self.clone()),
            self.config.supervisor_config.clone(),
        ))
    }

    /// Get the number of registered actors
    pub fn actor_count(&self) -> usize {
        let registry = self.registry.lock().unwrap();
        registry.len()
    }

    /// Shutdown the actor system
    pub fn shutdown(&self) {
        info!("Shutting down actor system");

        // First, mark all actors as stopping
        {
            let mut registry = self.registry.lock().unwrap();
            for (id, metadata) in registry.iter_mut() {
                debug!("Stopping actor: {}", id);
                metadata.status = ActorStatus::Stopping;
            }
        }

        // Shutdown the thread pool
        self.thread_pool.shutdown();

        // Mark all actors as stopped
        {
            let mut registry = self.registry.lock().unwrap();
            for (_, metadata) in registry.iter_mut() {
                metadata.status = ActorStatus::Stopped;
            }
        }

        info!("Actor system shutdown complete");
    }
}

impl Clone for ActorSystem {
    fn clone(&self) -> Self {
        // Note: This creates a new system with the same configuration
        // but does not share the registry. In a real implementation,
        // we would use Arc<Mutex<Registry>> to share the registry.
        Self::with_config(self.config.clone())
    }
}

impl Default for ActorSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_actor_registration() {
        let system = ActorSystem::new();

        // Register a root actor
        system.register_actor("actor1".into(), "TestActor", None);

        // Register a child actor
        system.register_actor("actor2".into(), "ChildActor", Some("actor1".into()));

        assert_eq!(system.actor_count(), 2);
        assert_eq!(
            system.get_status(&"actor1".into()),
            Some(ActorStatus::Starting)
        );
        assert_eq!(
            system.get_supervisor_id(&"actor2".into()),
            Some("actor1".into())
        );
    }

    #[test]
    fn test_status_updates() {
        let system = ActorSystem::new();

        system.register_actor("actor1".into(), "TestActor", None);

        // Update status
        system.update_status(&"actor1".into(), ActorStatus::Running);
        assert_eq!(
            system.get_status(&"actor1".into()),
            Some(ActorStatus::Running)
        );

        system.update_status(&"actor1".into(), ActorStatus::Crashed);
        assert_eq!(
            system.get_status(&"actor1".into()),
            Some(ActorStatus::Crashed)
        );
    }

    #[test]
    fn test_thread_pool_execution() {
        let system = ActorSystem::new();
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        system.submit(move || {
            thread::sleep(Duration::from_millis(10));
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
        });

        // Give time for the task to execute
        thread::sleep(Duration::from_millis(50));

        let count = *counter.lock().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_system_shutdown() {
        let system = ActorSystem::new();

        system.register_actor("actor1".into(), "TestActor", None);
        system.update_status(&"actor1".into(), ActorStatus::Running);

        system.shutdown();

        assert_eq!(
            system.get_status(&"actor1".into()),
            Some(ActorStatus::Stopped)
        );
    }
}
