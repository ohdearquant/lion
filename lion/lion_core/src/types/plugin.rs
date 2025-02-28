//! Plugin-related data types.
//!
//! This module defines data structures for plugin configuration, metadata,
//! state, and resource usage. These types are used throughout the system
//! to manage plugins, their lifecycle, and their resources.
//!
//! The plugin lifecycle is based on the "Concurrent Plugin Lifecycle Management
//! in Capability-Secured Microkernels" research.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::id::PluginId;

/// Plugin type.
///
/// This enum represents the different types of plugins that can be
/// loaded into the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginType {
    /// WebAssembly plugin, run in a WebAssembly VM.
    Wasm,

    /// Native plugin (shared library), run in the host process.
    Native,

    /// JavaScript plugin, run in a JavaScript VM.
    JavaScript,

    /// Remote plugin (in another process or node).
    Remote,
}

impl fmt::Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wasm => write!(f, "WebAssembly"),
            Self::Native => write!(f, "Native"),
            Self::JavaScript => write!(f, "JavaScript"),
            Self::Remote => write!(f, "Remote"),
        }
    }
}

/// Plugin state in the lifecycle.
///
/// This enum represents the different states a plugin can be in
/// during its lifecycle, from creation to termination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is created but not yet initialized.
    Created,

    /// Plugin is initialized and ready to run.
    Ready,

    /// Plugin is actively running.
    Running,

    /// Plugin is paused.
    Paused,

    /// Plugin has failed.
    Failed,

    /// Plugin has been terminated.
    Terminated,

    /// Plugin is upgrading (hot reload).
    Upgrading,
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Ready => write!(f, "Ready"),
            Self::Running => write!(f, "Running"),
            Self::Paused => write!(f, "Paused"),
            Self::Failed => write!(f, "Failed"),
            Self::Terminated => write!(f, "Terminated"),
            Self::Upgrading => write!(f, "Upgrading"),
        }
    }
}

impl PluginState {
    /// Check if this state represents an active plugin.
    ///
    /// # Returns
    ///
    /// `true` if the plugin is active, `false` otherwise.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Ready | Self::Running | Self::Paused | Self::Upgrading
        )
    }

    /// Check if this state allows function calls.
    ///
    /// # Returns
    ///
    /// `true` if function calls are allowed, `false` otherwise.
    pub fn allows_calls(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if this state allows state transitions.
    ///
    /// # Returns
    ///
    /// `true` if state transitions are allowed, `false` otherwise.
    pub fn allows_transitions(&self) -> bool {
        !matches!(self, Self::Terminated | Self::Failed)
    }

    /// Get the valid next states from this state.
    ///
    /// # Returns
    ///
    /// A vector of states that are valid transitions from this state.
    pub fn valid_next_states(&self) -> Vec<PluginState> {
        match self {
            Self::Created => vec![Self::Ready, Self::Failed, Self::Terminated],
            Self::Ready => vec![Self::Running, Self::Failed, Self::Terminated],
            Self::Running => vec![
                Self::Paused,
                Self::Ready,
                Self::Failed,
                Self::Terminated,
                Self::Upgrading,
            ],
            Self::Paused => vec![Self::Running, Self::Ready, Self::Failed, Self::Terminated],
            Self::Failed => vec![Self::Terminated],
            Self::Terminated => vec![],
            Self::Upgrading => vec![Self::Ready, Self::Failed, Self::Terminated],
        }
    }

    /// Check if a transition to the given state is valid.
    ///
    /// # Arguments
    ///
    /// * `next` - The state to transition to.
    ///
    /// # Returns
    ///
    /// `true` if the transition is valid, `false` otherwise.
    pub fn can_transition_to(&self, next: PluginState) -> bool {
        self.valid_next_states().contains(&next)
    }
}

/// Plugin configuration with resource limits.
///
/// This structure contains configuration options for a plugin,
/// including resource limits and other options.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Maximum memory usage in bytes.
    pub max_memory_bytes: Option<usize>,

    /// Maximum CPU time in microseconds.
    pub max_cpu_time_us: Option<u64>,

    /// Function call timeout in milliseconds.
    pub function_timeout_ms: Option<u64>,

    /// Maximum number of instances to keep ready.
    pub max_instances: Option<usize>,

    /// Minimum number of instances to keep ready.
    pub min_instances: Option<usize>,

    /// Whether to enable hot reloading.
    pub enable_hot_reload: Option<bool>,

    /// Whether to enable debug features.
    pub enable_debug: Option<bool>,

    /// Additional configuration options as a JSON object.
    pub options: serde_json::Value,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(100 * 1024 * 1024), // 100 MB
            max_cpu_time_us: Some(10 * 1000 * 1000),   // 10 seconds
            function_timeout_ms: Some(5000),           // 5 seconds
            max_instances: Some(10),
            min_instances: Some(1),
            enable_hot_reload: Some(true),
            enable_debug: Some(false),
            options: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

impl PluginConfig {
    /// Create a new plugin configuration with default values.
    ///
    /// # Returns
    ///
    /// A new plugin configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new minimal plugin configuration.
    ///
    /// This configuration has lower resource limits than the default,
    /// suitable for lightweight plugins.
    ///
    /// # Returns
    ///
    /// A new minimal plugin configuration.
    pub fn minimal() -> Self {
        Self {
            max_memory_bytes: Some(10 * 1024 * 1024), // 10 MB
            max_cpu_time_us: Some(1 * 1000 * 1000),   // 1 second
            function_timeout_ms: Some(1000),          // 1 second
            max_instances: Some(1),
            min_instances: Some(1),
            enable_hot_reload: Some(false),
            enable_debug: Some(false),
            options: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Get an option from the options object.
    ///
    /// # Arguments
    ///
    /// * `key` - The option key to get.
    ///
    /// # Returns
    ///
    /// The option value, or `None` if the option is not set.
    pub fn get_option<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        if let serde_json::Value::Object(ref map) = self.options {
            if let Some(value) = map.get(key) {
                return serde_json::from_value(value.clone()).ok();
            }
        }
        None
    }

    /// Set an option in the options object.
    ///
    /// # Arguments
    ///
    /// * `key` - The option key to set.
    /// * `value` - The option value to set.
    ///
    /// # Returns
    ///
    /// `true` if the option was set successfully, `false` otherwise.
    pub fn set_option<T: Serialize>(&mut self, key: &str, value: T) -> bool {
        if let serde_json::Value::Object(ref mut map) = self.options {
            match serde_json::to_value(value) {
                Ok(json_value) => {
                    map.insert(key.to_string(), json_value);
                    true
                }
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

/// Basic plugin metadata.
///
/// This structure contains metadata about a plugin, including
/// its ID, name, version, and other information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin ID.
    pub id: PluginId,

    /// Human-readable name.
    pub name: String,

    /// Version string.
    pub version: String,

    /// Description of the plugin.
    pub description: String,

    /// Plugin type.
    pub plugin_type: PluginType,

    /// Current state.
    pub state: PluginState,

    /// When the plugin was created.
    pub created_at: DateTime<Utc>,

    /// When the plugin was last updated.
    pub updated_at: DateTime<Utc>,

    /// Available functions.
    pub functions: Vec<String>,
}

impl PluginMetadata {
    /// Create new plugin metadata.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name.
    /// * `version` - Version string.
    /// * `description` - Description of the plugin.
    /// * `plugin_type` - Plugin type.
    ///
    /// # Returns
    ///
    /// New plugin metadata with a unique ID and default state.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        description: impl Into<String>,
        plugin_type: PluginType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: PluginId::new(),
            name: name.into(),
            version: version.into(),
            description: description.into(),
            plugin_type,
            state: PluginState::Created,
            created_at: now,
            updated_at: now,
            functions: Vec::new(),
        }
    }

    /// Update the plugin state.
    ///
    /// # Arguments
    ///
    /// * `state` - The new state.
    ///
    /// # Returns
    ///
    /// `true` if the state was updated, `false` if the transition is invalid.
    pub fn update_state(&mut self, state: PluginState) -> bool {
        if self.state.can_transition_to(state) {
            self.state = state;
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Add a function to the list of available functions.
    ///
    /// # Arguments
    ///
    /// * `function` - The function name to add.
    pub fn add_function(&mut self, function: impl Into<String>) {
        self.functions.push(function.into());
        self.updated_at = Utc::now();
    }

    /// Set the list of available functions.
    ///
    /// # Arguments
    ///
    /// * `functions` - The list of function names.
    pub fn set_functions(&mut self, functions: Vec<String>) {
        self.functions = functions;
        self.updated_at = Utc::now();
    }
}

/// Resource usage statistics for a plugin.
///
/// This structure contains resource usage statistics for a plugin,
/// including memory usage, CPU time, and other metrics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in bytes.
    pub memory_bytes: usize,

    /// CPU time used in microseconds.
    pub cpu_time_us: u64,

    /// Number of function calls executed.
    pub function_calls: u64,

    /// Number of instances currently active.
    pub active_instances: usize,

    /// Peak memory usage in bytes.
    pub peak_memory_bytes: usize,

    /// Peak CPU time in microseconds.
    pub peak_cpu_time_us: u64,

    /// Average function call duration in microseconds.
    pub avg_function_call_us: u64,

    /// Custom metrics.
    pub custom_metrics: HashMap<String, f64>,

    /// When resource usage was last updated.
    pub last_updated: DateTime<Utc>,
}

impl ResourceUsage {
    /// Create new resource usage statistics.
    ///
    /// # Returns
    ///
    /// New resource usage statistics with zero values.
    pub fn new() -> Self {
        Self {
            last_updated: Utc::now(),
            ..Default::default()
        }
    }

    /// Update memory usage.
    ///
    /// # Arguments
    ///
    /// * `memory_bytes` - Current memory usage in bytes.
    pub fn update_memory(&mut self, memory_bytes: usize) {
        self.memory_bytes = memory_bytes;
        self.peak_memory_bytes = self.peak_memory_bytes.max(memory_bytes);
        self.last_updated = Utc::now();
    }

    /// Update CPU time.
    ///
    /// # Arguments
    ///
    /// * `cpu_time_us` - Current CPU time used in microseconds.
    pub fn update_cpu_time(&mut self, cpu_time_us: u64) {
        self.cpu_time_us = cpu_time_us;
        self.peak_cpu_time_us = self.peak_cpu_time_us.max(cpu_time_us);
        self.last_updated = Utc::now();
    }

    /// Record a function call.
    ///
    /// # Arguments
    ///
    /// * `duration_us` - Duration of the function call in microseconds.
    pub fn record_function_call(&mut self, duration_us: u64) {
        self.function_calls += 1;

        // Update average function call duration
        if self.function_calls == 1 {
            self.avg_function_call_us = duration_us;
        } else {
            // Rolling average
            self.avg_function_call_us = (self.avg_function_call_us * (self.function_calls - 1)
                + duration_us)
                / self.function_calls;
        }

        self.last_updated = Utc::now();
    }

    /// Set the number of active instances.
    ///
    /// # Arguments
    ///
    /// * `active_instances` - Number of instances currently active.
    pub fn set_active_instances(&mut self, active_instances: usize) {
        self.active_instances = active_instances;
        self.last_updated = Utc::now();
    }

    /// Set a custom metric.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the metric.
    /// * `value` - Value of the metric.
    pub fn set_custom_metric(&mut self, name: impl Into<String>, value: f64) {
        self.custom_metrics.insert(name.into(), value);
        self.last_updated = Utc::now();
    }

    /// Get a custom metric.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the metric.
    ///
    /// # Returns
    ///
    /// The metric value, or `None` if the metric is not set.
    pub fn get_custom_metric(&self, name: &str) -> Option<f64> {
        self.custom_metrics.get(name).cloned()
    }

    /// Reset all metrics to zero.
    pub fn reset(&mut self) {
        self.memory_bytes = 0;
        self.cpu_time_us = 0;
        self.function_calls = 0;
        self.active_instances = 0;
        self.peak_memory_bytes = 0;
        self.peak_cpu_time_us = 0;
        self.avg_function_call_us = 0;
        self.custom_metrics.clear();
        self.last_updated = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_type_display() {
        assert_eq!(PluginType::Wasm.to_string(), "WebAssembly");
        assert_eq!(PluginType::Native.to_string(), "Native");
        assert_eq!(PluginType::JavaScript.to_string(), "JavaScript");
        assert_eq!(PluginType::Remote.to_string(), "Remote");
    }

    #[test]
    fn test_plugin_state_display() {
        assert_eq!(PluginState::Created.to_string(), "Created");
        assert_eq!(PluginState::Ready.to_string(), "Ready");
        assert_eq!(PluginState::Running.to_string(), "Running");
        assert_eq!(PluginState::Paused.to_string(), "Paused");
        assert_eq!(PluginState::Failed.to_string(), "Failed");
        assert_eq!(PluginState::Terminated.to_string(), "Terminated");
        assert_eq!(PluginState::Upgrading.to_string(), "Upgrading");
    }

    #[test]
    fn test_plugin_state_methods() {
        // Test is_active
        assert!(!PluginState::Created.is_active());
        assert!(PluginState::Ready.is_active());
        assert!(PluginState::Running.is_active());
        assert!(PluginState::Paused.is_active());
        assert!(!PluginState::Failed.is_active());
        assert!(!PluginState::Terminated.is_active());
        assert!(PluginState::Upgrading.is_active());

        // Test allows_calls
        assert!(!PluginState::Created.allows_calls());
        assert!(!PluginState::Ready.allows_calls());
        assert!(PluginState::Running.allows_calls());
        assert!(!PluginState::Paused.allows_calls());
        assert!(!PluginState::Failed.allows_calls());
        assert!(!PluginState::Terminated.allows_calls());
        assert!(!PluginState::Upgrading.allows_calls());

        // Test allows_transitions
        assert!(PluginState::Created.allows_transitions());
        assert!(PluginState::Ready.allows_transitions());
        assert!(PluginState::Running.allows_transitions());
        assert!(PluginState::Paused.allows_transitions());
        assert!(!PluginState::Failed.allows_transitions());
        assert!(!PluginState::Terminated.allows_transitions());
        assert!(PluginState::Upgrading.allows_transitions());

        // Test valid_next_states
        assert!(PluginState::Created
            .valid_next_states()
            .contains(&PluginState::Ready));
        assert!(PluginState::Ready
            .valid_next_states()
            .contains(&PluginState::Running));
        assert!(PluginState::Running
            .valid_next_states()
            .contains(&PluginState::Paused));
        assert!(PluginState::Paused
            .valid_next_states()
            .contains(&PluginState::Running));
        assert!(PluginState::Failed
            .valid_next_states()
            .contains(&PluginState::Terminated));
        assert!(PluginState::Terminated.valid_next_states().is_empty());

        // Test can_transition_to
        assert!(PluginState::Created.can_transition_to(PluginState::Ready));
        assert!(PluginState::Ready.can_transition_to(PluginState::Running));
        assert!(PluginState::Running.can_transition_to(PluginState::Paused));
        assert!(PluginState::Paused.can_transition_to(PluginState::Running));
        assert!(PluginState::Failed.can_transition_to(PluginState::Terminated));

        assert!(!PluginState::Created.can_transition_to(PluginState::Running));
        assert!(!PluginState::Ready.can_transition_to(PluginState::Paused));
        assert!(!PluginState::Terminated.can_transition_to(PluginState::Ready));
    }

    #[test]
    fn test_plugin_config() {
        // Test default configuration
        let config = PluginConfig::default();
        assert_eq!(config.max_memory_bytes, Some(100 * 1024 * 1024));
        assert_eq!(config.max_cpu_time_us, Some(10 * 1000 * 1000));
        assert_eq!(config.function_timeout_ms, Some(5000));

        // Test minimal configuration
        let minimal = PluginConfig::minimal();
        assert_eq!(minimal.max_memory_bytes, Some(10 * 1024 * 1024));
        assert_eq!(minimal.max_cpu_time_us, Some(1 * 1000 * 1000));
        assert_eq!(minimal.function_timeout_ms, Some(1000));

        // Test options
        let mut config = PluginConfig::default();
        assert!(config.set_option("test_key", "test_value"));
        assert_eq!(
            config.get_option::<String>("test_key").unwrap(),
            "test_value"
        );

        assert!(config.set_option("test_number", 42));
        assert_eq!(config.get_option::<i32>("test_number").unwrap(), 42);

        assert_eq!(config.get_option::<String>("nonexistent"), None);
    }

    #[test]
    fn test_plugin_metadata() {
        // Test constructor
        let metadata =
            PluginMetadata::new("Test Plugin", "1.0.0", "A test plugin", PluginType::Wasm);

        assert_eq!(metadata.name, "Test Plugin");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, "A test plugin");
        assert_eq!(metadata.plugin_type, PluginType::Wasm);
        assert_eq!(metadata.state, PluginState::Created);
        assert!(metadata.functions.is_empty());

        // Test update_state
        let mut metadata = metadata.clone();
        assert!(metadata.update_state(PluginState::Ready));
        assert_eq!(metadata.state, PluginState::Ready);

        // Test invalid state transition
        assert!(!metadata.update_state(PluginState::Paused));
        assert_eq!(metadata.state, PluginState::Ready);

        // Test add_function and set_functions
        let mut metadata = metadata.clone();
        metadata.add_function("test_function");
        assert_eq!(metadata.functions, vec!["test_function"]);

        metadata.set_functions(vec!["function1".to_string(), "function2".to_string()]);
        assert_eq!(metadata.functions, vec!["function1", "function2"]);
    }

    #[test]
    fn test_resource_usage() {
        // Test constructor
        let usage = ResourceUsage::new();
        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.cpu_time_us, 0);
        assert_eq!(usage.function_calls, 0);

        // Test update_memory
        let mut usage = usage.clone();
        usage.update_memory(1024);
        assert_eq!(usage.memory_bytes, 1024);
        assert_eq!(usage.peak_memory_bytes, 1024);

        usage.update_memory(2048);
        assert_eq!(usage.memory_bytes, 2048);
        assert_eq!(usage.peak_memory_bytes, 2048);

        usage.update_memory(1024);
        assert_eq!(usage.memory_bytes, 1024);
        assert_eq!(usage.peak_memory_bytes, 2048);

        // Test update_cpu_time
        let mut usage = usage.clone();
        usage.update_cpu_time(1000);
        assert_eq!(usage.cpu_time_us, 1000);
        assert_eq!(usage.peak_cpu_time_us, 1000);

        usage.update_cpu_time(2000);
        assert_eq!(usage.cpu_time_us, 2000);
        assert_eq!(usage.peak_cpu_time_us, 2000);

        // Test record_function_call
        let mut usage = usage.clone();
        usage.record_function_call(1000);
        assert_eq!(usage.function_calls, 1);
        assert_eq!(usage.avg_function_call_us, 1000);

        usage.record_function_call(3000);
        assert_eq!(usage.function_calls, 2);
        assert_eq!(usage.avg_function_call_us, 2000);

        // Test set_active_instances
        let mut usage = usage.clone();
        usage.set_active_instances(5);
        assert_eq!(usage.active_instances, 5);

        // Test custom metrics
        let mut usage = usage.clone();
        usage.set_custom_metric("test_metric", 42.0);
        assert_eq!(usage.get_custom_metric("test_metric"), Some(42.0));
        assert_eq!(usage.get_custom_metric("nonexistent"), None);

        // Test reset
        let mut usage = usage.clone();
        usage.update_memory(1024);
        usage.update_cpu_time(1000);
        usage.record_function_call(1000);
        usage.set_active_instances(5);
        usage.set_custom_metric("test_metric", 42.0);

        usage.reset();
        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.cpu_time_us, 0);
        assert_eq!(usage.function_calls, 0);
        assert_eq!(usage.active_instances, 0);
        assert_eq!(usage.peak_memory_bytes, 0);
        assert_eq!(usage.peak_cpu_time_us, 0);
        assert_eq!(usage.avg_function_call_us, 0);
        assert!(usage.custom_metrics.is_empty());
    }

    #[test]
    fn test_serialization() {
        // Test PluginType serialization
        let plugin_type = PluginType::Wasm;
        let serialized = serde_json::to_string(&plugin_type).unwrap();
        let deserialized: PluginType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(plugin_type, deserialized);

        // Test PluginState serialization
        let plugin_state = PluginState::Running;
        let serialized = serde_json::to_string(&plugin_state).unwrap();
        let deserialized: PluginState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(plugin_state, deserialized);

        // Test PluginConfig serialization
        let config = PluginConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: PluginConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config.max_memory_bytes, deserialized.max_memory_bytes);
        assert_eq!(config.max_cpu_time_us, deserialized.max_cpu_time_us);
        assert_eq!(config.function_timeout_ms, deserialized.function_timeout_ms);

        // Test PluginMetadata serialization
        let metadata =
            PluginMetadata::new("Test Plugin", "1.0.0", "A test plugin", PluginType::Wasm);
        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: PluginMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.name, deserialized.name);
        assert_eq!(metadata.version, deserialized.version);
        assert_eq!(metadata.description, deserialized.description);
        assert_eq!(metadata.plugin_type, deserialized.plugin_type);
        assert_eq!(metadata.state, deserialized.state);

        // Test ResourceUsage serialization
        let mut usage = ResourceUsage::new();
        usage.update_memory(1024);
        usage.update_cpu_time(1000);
        usage.record_function_call(1000);
        usage.set_active_instances(5);
        usage.set_custom_metric("test_metric", 42.0);

        let serialized = serde_json::to_string(&usage).unwrap();
        let deserialized: ResourceUsage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(usage.memory_bytes, deserialized.memory_bytes);
        assert_eq!(usage.cpu_time_us, deserialized.cpu_time_us);
        assert_eq!(usage.function_calls, deserialized.function_calls);
        assert_eq!(usage.active_instances, deserialized.active_instances);
        assert_eq!(usage.peak_memory_bytes, deserialized.peak_memory_bytes);
        assert_eq!(usage.peak_cpu_time_us, deserialized.peak_cpu_time_us);
        assert_eq!(
            usage.avg_function_call_us,
            deserialized.avg_function_call_us
        );
        assert_eq!(
            usage.get_custom_metric("test_metric"),
            deserialized.get_custom_metric("test_metric")
        );
    }
}
