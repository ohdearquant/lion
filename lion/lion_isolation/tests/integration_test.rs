//! Integration tests for lion_isolation.

use lion_core::error::{IsolationError, Result};
use lion_core::id::PluginId;
use lion_core::types::ResourceUsage;
use lion_isolation::*;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Initialize tracing for tests
fn init_tracing() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}

// A mock isolation backend for testing
struct MockIsolationBackend {
    plugin_id: PluginId,
    state: PluginState,
}

impl MockIsolationBackend {
    fn new() -> Self {
        Self {
            plugin_id: PluginId::new(),
            state: PluginState::Loaded,
        }
    }
}

impl IsolationBackend for MockIsolationBackend {
    fn load_plugin(&mut self, plugin_id: &PluginId, _wasm_bytes: &[u8]) -> Result<()> {
        self.plugin_id = plugin_id.clone();
        self.state = PluginState::Loaded;
        Ok(())
    }

    fn unload_plugin(&mut self, plugin_id: &PluginId) -> Result<()> {
        if *plugin_id == self.plugin_id {
            self.state = PluginState::Unloaded;
            Ok(())
        } else {
            Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
        }
    }

    fn call_function(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        if *plugin_id != self.plugin_id {
            return Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into());
        }

        if self.state != PluginState::Loaded && self.state != PluginState::Running {
            return Err(IsolationError::ExecutionTrap(format!(
                "Plugin {} is not in a runnable state",
                plugin_id
            ))
            .into());
        }

        // Mock implementation for "add" function
        if function_name == "add" && params.len() >= 2 {
            let result = params[0] + params[1];
            Ok(vec![result])
        } else {
            Ok(vec![42]) // Default response for any other function
        }
    }

    fn get_plugin_state(&self, plugin_id: &PluginId) -> Result<PluginState> {
        if *plugin_id == self.plugin_id {
            if self.state == PluginState::Unloaded {
                Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
            } else {
                Ok(self.state)
            }
        } else {
            Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
        }
    }

    fn get_resource_usage(&self, plugin_id: &PluginId) -> Result<ResourceUsage> {
        if *plugin_id == self.plugin_id {
            Ok(ResourceUsage::default())
        } else {
            Err(IsolationError::PluginNotLoaded(plugin_id.clone()).into())
        }
    }

    fn set_capability_checker(&mut self, _checker: Box<dyn interface::CapabilityChecker>) {
        // No-op for mock
    }
}

#[test]
fn test_isolation_workflow() {
    // Initialize tracing
    init_tracing();

    // Create a mock backend
    let backend = MockIsolationBackend::new();

    // Create an isolation manager with the mock backend
    let mut manager = IsolationManager::new(Box::new(backend));

    // Create a plugin ID
    let plugin_id = PluginId::new();

    // Create dummy WASM bytes
    let wasm_bytes = [0, 1, 2, 3];

    // Load the plugin
    manager.load_plugin(&plugin_id, &wasm_bytes).unwrap();

    // Get the plugin state
    let state = manager.get_plugin_state(&plugin_id).unwrap();
    assert_eq!(state, PluginState::Loaded);

    // Call a function
    info!("Calling add function");
    let result = manager.call_function(&plugin_id, "add", &[1, 2]).unwrap();
    assert_eq!(result, vec![3]);

    // Get the resource usage
    let usage = manager.get_resource_usage(&plugin_id).unwrap();
    info!("Resource usage: {:?}", usage);

    // Unload the plugin
    manager.unload_plugin(&plugin_id).unwrap();

    // Check that the plugin is gone
    assert!(manager.get_plugin_state(&plugin_id).is_err());
}
