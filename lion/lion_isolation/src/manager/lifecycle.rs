//! Plugin lifecycle management.
//!
//! This module provides functionality for managing plugin lifecycles.

use lion_core::PluginId;
use std::time::Instant;

/// A plugin state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// The plugin is loading.
    Loading,

    /// The plugin is loaded.
    Loaded,

    /// The plugin is running.
    Running,

    /// The plugin is paused.
    Paused,

    /// The plugin has failed.
    Failed,

    /// The plugin is unloaded.
    Unloaded,
}

/// A plugin lifecycle.
#[derive(Debug, Clone)]
pub struct PluginLifecycle {
    /// The plugin ID.
    plugin_id: PluginId,

    /// The plugin state.
    state: PluginState,

    /// When the plugin was created.
    created_at: Instant,

    /// When the plugin last changed state.
    last_state_change: Instant,

    /// The number of function calls.
    function_calls: u64,
}

impl PluginLifecycle {
    /// Create a new plugin lifecycle.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `state` - The initial state.
    ///
    /// # Returns
    ///
    /// A new plugin lifecycle.
    pub fn new(plugin_id: PluginId, state: PluginState) -> Self {
        let now = Instant::now();

        Self {
            plugin_id,
            state,
            created_at: now,
            last_state_change: now,
            function_calls: 0,
        }
    }

    /// Get the plugin ID.
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Get the plugin state.
    pub fn state(&self) -> PluginState {
        self.state
    }

    /// Get when the plugin was created.
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Get when the plugin last changed state.
    pub fn last_state_change(&self) -> Instant {
        self.last_state_change
    }

    /// Get the number of function calls.
    pub fn function_calls(&self) -> u64 {
        self.function_calls
    }

    /// Transition to a new state.
    ///
    /// # Arguments
    ///
    /// * `state` - The new state.
    pub fn transition_to(&mut self, state: PluginState) {
        self.state = state;
        self.last_state_change = Instant::now();
    }

    /// Check if the plugin can call a function.
    ///
    /// # Returns
    ///
    /// `true` if the plugin can call a function, `false` otherwise.
    pub fn can_call_function(&self) -> bool {
        matches!(self.state, PluginState::Loaded | PluginState::Running)
    }

    /// Increment the function call count.
    pub fn increment_function_calls(&mut self) {
        self.function_calls += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_plugin_lifecycle() {
        // Create a plugin lifecycle
        let plugin_id = "test-plugin".to_string();
        let mut lifecycle = PluginLifecycle::new(plugin_id.clone(), PluginState::Loading);

        // Check initial state
        assert_eq!(lifecycle.plugin_id(), &plugin_id);
        assert_eq!(lifecycle.state(), PluginState::Loading);
        assert_eq!(lifecycle.function_calls(), 0);

        // Sleep a bit
        sleep(Duration::from_millis(10));

        // Transition to Loaded
        lifecycle.transition_to(PluginState::Loaded);

        // Check state change
        assert_eq!(lifecycle.state(), PluginState::Loaded);
        assert!(lifecycle.last_state_change() > lifecycle.created_at());

        // Check function calling
        assert!(lifecycle.can_call_function());

        // Increment function calls
        lifecycle.increment_function_calls();
        assert_eq!(lifecycle.function_calls(), 1);

        // Transition to Failed
        lifecycle.transition_to(PluginState::Failed);

        // Check function calling again
        assert!(!lifecycle.can_call_function());
    }
}
