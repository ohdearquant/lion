//! Capability-based security integration for observability
//!
//! This module defines the capabilities required for observability operations
//! and provides integration with Lion's capability system.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents specific observability capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservabilityCapability {
    /// Capability to write logs at different levels
    Log(LogLevel),

    /// Capability to create and record spans
    Tracing,

    /// Capability to record metrics
    Metrics,

    /// Capability to view logs from other plugins
    ViewLogs,

    /// Capability to view traces from other plugins
    ViewTraces,

    /// Capability to view metrics from other plugins
    ViewMetrics,

    /// Capability to configure observability settings
    Configure,
}

/// Log level for capability checking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Trace level logs (most verbose)
    Trace,
    /// Debug level logs
    Debug,
    /// Info level logs
    Info,
    /// Warning level logs
    Warn,
    /// Error level logs
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" | "warning" => LogLevel::Warn,
            "error" | "err" => LogLevel::Error,
            _ => LogLevel::Info, // Default to Info
        }
    }
}

impl fmt::Display for ObservabilityCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObservabilityCapability::Log(level) => write!(f, "log:{}", level),
            ObservabilityCapability::Tracing => write!(f, "tracing"),
            ObservabilityCapability::Metrics => write!(f, "metrics"),
            ObservabilityCapability::ViewLogs => write!(f, "view_logs"),
            ObservabilityCapability::ViewTraces => write!(f, "view_traces"),
            ObservabilityCapability::ViewMetrics => write!(f, "view_metrics"),
            ObservabilityCapability::Configure => write!(f, "configure"),
        }
    }
}

/// Trait for checking observability capabilities
pub trait ObservabilityCapabilityChecker: Send + Sync {
    /// Check if the specified plugin has the given capability
    fn check_capability(
        &self,
        plugin_id: &str,
        capability: ObservabilityCapability,
    ) -> Result<bool>;

    /// Get the name of this capability checker
    fn name(&self) -> &str;
}

/// Default implementation that always allows capabilities
#[derive(Debug, Clone)]
pub struct AllowAllCapabilityChecker;

impl ObservabilityCapabilityChecker for AllowAllCapabilityChecker {
    fn check_capability(
        &self,
        _plugin_id: &str,
        _capability: ObservabilityCapability,
    ) -> Result<bool> {
        Ok(true)
    }

    fn name(&self) -> &str {
        "allow_all"
    }
}

/// Implementation that always denies capabilities
#[derive(Debug, Clone)]
pub struct DenyAllCapabilityChecker;

impl ObservabilityCapabilityChecker for DenyAllCapabilityChecker {
    fn check_capability(
        &self,
        _plugin_id: &str,
        _capability: ObservabilityCapability,
    ) -> Result<bool> {
        Ok(false)
    }

    fn name(&self) -> &str {
        "deny_all"
    }
}

/// Capability checker that selectively allows specific capabilities
#[derive(Debug, Clone)]
pub struct SelectiveCapabilityChecker {
    name: String,
    allowed_capabilities: Vec<(String, ObservabilityCapability)>,
}

impl SelectiveCapabilityChecker {
    /// Create a new selective capability checker
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            allowed_capabilities: Vec::new(),
        }
    }

    /// Allow a specific capability for a plugin
    pub fn allow(
        mut self,
        plugin_id: impl Into<String>,
        capability: ObservabilityCapability,
    ) -> Self {
        self.allowed_capabilities
            .push((plugin_id.into(), capability));
        self
    }
}

impl ObservabilityCapabilityChecker for SelectiveCapabilityChecker {
    fn check_capability(
        &self,
        plugin_id: &str,
        capability: ObservabilityCapability,
    ) -> Result<bool> {
        // Special case for Log capability
        if let ObservabilityCapability::Log(requested_level) = capability {
            for (pid, cap) in &self.allowed_capabilities {
                if pid == plugin_id {
                    if let ObservabilityCapability::Log(allowed_level) = cap {
                        // The plugin can log at the requested level if it has permission
                        // for that level or any lower (more verbose) level
                        return Ok(requested_level >= *allowed_level);
                    }
                }
            }
        }

        Ok(self
            .allowed_capabilities
            .iter()
            .any(|(pid, cap)| pid == plugin_id && cap == &capability))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(feature = "capability-integration")]
/// Integration with Lion's capability system
pub mod lion_integration {
    use super::*;
    use lion_capability::store::CapabilityStore;
    use lion_capability::Capability;

    /// Capability checker that integrates with Lion's capability system
    #[derive(Debug, Clone)]
    pub struct LionCapabilityChecker {
        store: Arc<dyn CapabilityStore>,
    }

    impl LionCapabilityChecker {
        /// Create a new capability checker using Lion's capability store
        pub fn new(store: Arc<dyn CapabilityStore>) -> Self {
            Self { store }
        }
    }

    impl ObservabilityCapabilityChecker for LionCapabilityChecker {
        fn check_capability(
            &self,
            plugin_id: &str,
            capability: ObservabilityCapability,
        ) -> Result<bool> {
            let cap_string = capability.to_string();
            let lion_capability = Capability::new("observability", &cap_string);

            // Check the capability store
            Ok(self
                .store
                .check_capability(plugin_id, &lion_capability)
                .map_err(|e| ObservabilityError::CapabilityError(e.to_string()))?)
        }

        fn name(&self) -> &str {
            "lion_capability"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_all_checker() {
        let checker = AllowAllCapabilityChecker;
        assert!(checker
            .check_capability("plugin1", ObservabilityCapability::Log(LogLevel::Debug))
            .unwrap());
        assert!(checker
            .check_capability("plugin1", ObservabilityCapability::Metrics)
            .unwrap());
    }

    #[test]
    fn test_deny_all_checker() {
        let checker = DenyAllCapabilityChecker;
        assert!(!checker
            .check_capability("plugin1", ObservabilityCapability::Log(LogLevel::Debug))
            .unwrap());
        assert!(!checker
            .check_capability("plugin1", ObservabilityCapability::Metrics)
            .unwrap());
    }

    #[test]
    fn test_selective_checker() {
        let checker = SelectiveCapabilityChecker::new("test")
            .allow("plugin1", ObservabilityCapability::Log(LogLevel::Info))
            .allow("plugin2", ObservabilityCapability::Metrics);

        // plugin1 can log at Info level or higher (Error)
        assert!(checker
            .check_capability("plugin1", ObservabilityCapability::Log(LogLevel::Info))
            .unwrap());
        assert!(checker
            .check_capability("plugin1", ObservabilityCapability::Log(LogLevel::Error))
            .unwrap());

        // plugin1 cannot log at Debug or Trace (more verbose than Info)
        assert!(!checker
            .check_capability("plugin1", ObservabilityCapability::Log(LogLevel::Debug))
            .unwrap());
        assert!(!checker
            .check_capability("plugin1", ObservabilityCapability::Log(LogLevel::Trace))
            .unwrap());

        // plugin1 cannot use metrics
        assert!(!checker
            .check_capability("plugin1", ObservabilityCapability::Metrics)
            .unwrap());

        // plugin2 can use metrics
        assert!(checker
            .check_capability("plugin2", ObservabilityCapability::Metrics)
            .unwrap());

        // plugin2 cannot log
        assert!(!checker
            .check_capability("plugin2", ObservabilityCapability::Log(LogLevel::Error))
            .unwrap());
    }
}
