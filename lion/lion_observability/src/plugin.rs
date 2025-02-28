//! Plugin observability management
//!
//! This module provides observability functionality for plugins
//! with capability-based access control and context propagation.

use std::collections::HashMap;
use std::sync::Arc;

use crate::capability::{LogLevel, ObservabilityCapability, ObservabilityCapabilityChecker};
use crate::context::{Context, SpanContext};
use crate::error::ObservabilityError;
use crate::logging::SimpleLogger;
use crate::logging::{LogEvent, LoggerBase};
use crate::metrics::{Counter, Gauge, Histogram, MetricsRegistry};
use crate::tracing_system::NoopTracer;
use crate::tracing_system::{Span, SpanStatus, TracerBase, TracingEvent};
use crate::Result;

/// Observability for a specific plugin
#[derive(Clone)]
pub struct PluginObservability {
    /// Plugin ID
    plugin_id: String,

    /// Logger
    logger: Arc<SimpleLogger>,

    /// Tracer
    tracer: Arc<NoopTracer>,

    /// Metrics registry
    metrics_registry: Arc<dyn MetricsRegistry>,

    /// Capability checker (optional)
    #[allow(dead_code)]
    capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,

    /// Whether to enforce capabilities
    enforce_capabilities: bool,
}

impl PluginObservability {
    /// Create a new plugin observability instance
    pub fn new(
        plugin_id: String,
        logger: Arc<SimpleLogger>,
        tracer: Arc<NoopTracer>,
        metrics_registry: Arc<dyn MetricsRegistry>,
        capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,
        enforce_capabilities: bool,
    ) -> Self {
        Self {
            plugin_id,
            logger,
            tracer,
            metrics_registry,
            capability_checker,
            enforce_capabilities,
        }
    }

    /// Start a span with the plugin context
    pub fn start_span(&self, name: impl Into<String>) -> Result<Span> {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability =
                    checker.check_capability(&self.plugin_id, ObservabilityCapability::Tracing)?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(
                        "Missing tracing capability".to_string(),
                    ));
                }
            }
        }

        // Create the span
        let name_str = name.into();
        let mut span = self.tracer.create_span_with_name(&name_str)?;

        // Add plugin ID as an attribute
        span = span.with_attribute("plugin_id", self.plugin_id.clone())?;

        Ok(span)
    }

    /// Run a function within a span
    pub fn with_span<F, R>(&self, name: impl Into<String>, f: F) -> Result<R>
    where
        F: FnOnce() -> R,
    {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability =
                    checker.check_capability(&self.plugin_id, ObservabilityCapability::Tracing)?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(
                        "Missing tracing capability".to_string(),
                    ));
                }
            }
        }

        let name_str = name.into();

        // Capture the "parent" context
        let parent_ctx = Context::current().unwrap_or_default();

        // Create a new span context (either as a child or a root)
        let new_span_ctx = if let Some(ref parent_span_ctx) = parent_ctx.span_context {
            // Create a child span context from the parent
            parent_span_ctx.new_child(&name_str)
        } else {
            // Create a new root span context
            SpanContext::new_root(&name_str)
        };

        // Create a child context with the plugin ID and the new span context
        let child_ctx = parent_ctx
            .with_plugin_id(self.plugin_id.clone())
            .with_span_context(new_span_ctx);

        // Run with the child context - implement with_span manually since we can't use the Tracer trait directly
        child_ctx.with_current(|| {
            // Create a new span
            let mut span = match &child_ctx.span_context {
                Some(span_ctx) => {
                    // Use the span context's name and IDs for the span
                    let mut s = self.tracer.create_span_with_name(&span_ctx.name)?;
                    s = s.with_attribute("trace_id", span_ctx.trace_id.clone())?;
                    s = s.with_attribute("span_id", span_ctx.span_id.clone())?;
                    if let Some(parent_id) = &span_ctx.parent_span_id {
                        s = s.with_attribute("parent_span_id", parent_id.clone())?;
                    }
                    s
                }
                None => self.tracer.create_span_with_name(&name_str)?,
            };

            // Add plugin ID as an attribute
            span = span.with_attribute("plugin_id", self.plugin_id.clone())?;

            // Execute the function
            let result = f();

            // End the span
            span.end();
            self.tracer.record_span(span)?;

            Ok(result)
        })
    }

    /// Create or get a counter
    pub fn counter(
        &self,
        name: &str,
        description: &str,
        labels: HashMap<String, String>,
    ) -> Result<Arc<dyn Counter>> {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability =
                    checker.check_capability(&self.plugin_id, ObservabilityCapability::Metrics)?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(
                        "Missing metrics capability".to_string(),
                    ));
                }
            }
        }

        // Add plugin ID to labels
        let mut labels = labels;
        labels.insert("plugin_id".to_string(), self.plugin_id.clone());

        // Create the counter
        self.metrics_registry.counter(name, description, labels)
    }

    /// Create or get a gauge
    pub fn gauge(
        &self,
        name: &str,
        description: &str,
        labels: HashMap<String, String>,
    ) -> Result<Arc<dyn Gauge>> {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability =
                    checker.check_capability(&self.plugin_id, ObservabilityCapability::Metrics)?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(
                        "Missing metrics capability".to_string(),
                    ));
                }
            }
        }

        // Add plugin ID to labels
        let mut labels = labels;
        labels.insert("plugin_id".to_string(), self.plugin_id.clone());

        // Create the gauge
        self.metrics_registry.gauge(name, description, labels)
    }

    /// Create or get a histogram
    pub fn histogram(
        &self,
        name: &str,
        description: &str,
        labels: HashMap<String, String>,
    ) -> Result<Arc<dyn Histogram>> {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability =
                    checker.check_capability(&self.plugin_id, ObservabilityCapability::Metrics)?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(
                        "Missing metrics capability".to_string(),
                    ));
                }
            }
        }

        // Add plugin ID to labels
        let mut labels = labels;
        labels.insert("plugin_id".to_string(), self.plugin_id.clone());

        // Create the histogram
        self.metrics_registry.histogram(name, description, labels)
    }

    /// Log a message at the specified level
    pub fn log(&self, level: LogLevel, message: impl Into<String>) -> Result<()> {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability = checker
                    .check_capability(&self.plugin_id, ObservabilityCapability::Log(level))?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(format!(
                        "Missing capability to log at {:?} level",
                        level
                    )));
                }
            }
        }

        // Create the log event
        let mut event = LogEvent::new(level, message.into());

        // Add plugin ID if not already present
        if event.plugin_id.is_none() {
            event.plugin_id = Some(self.plugin_id.clone());
        }

        // Add trace context if available
        if let Some(ctx) = Context::current() {
            if let Some(span_ctx) = &ctx.span_context {
                event.trace_id = Some(span_ctx.trace_id.clone());
                event.span_id = Some(span_ctx.span_id.clone());
            }
        }

        // Log the event
        self.logger.log(event)
    }

    /// Log at trace level
    pub fn trace(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Trace, message)
    }

    /// Log at debug level
    pub fn debug(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Debug, message)
    }

    /// Log at info level
    pub fn info(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Info, message)
    }

    /// Log at warn level
    pub fn warn(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Warn, message)
    }

    /// Log at error level
    pub fn error(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Error, message)
    }

    /// Get the plugin ID
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Create a context with the plugin ID
    pub fn create_context(&self) -> Context {
        // Create a context with plugin ID and a root span
        let span_context = SpanContext::new_root(&format!("root-{}", self.plugin_id));
        Context::new()
            .with_plugin_id(self.plugin_id.clone())
            .with_span_context(span_context)
    }

    /// Create a context with the plugin ID and span
    pub fn create_context_with_span(&self, span_context: SpanContext) -> Context {
        Context::new()
            .with_plugin_id(self.plugin_id.clone())
            .with_span_context(span_context)
    }

    /// Add an event to the current span
    pub fn add_span_event(&self, event: TracingEvent) -> Result<()> {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability =
                    checker.check_capability(&self.plugin_id, ObservabilityCapability::Tracing)?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(
                        "Missing tracing capability".to_string(),
                    ));
                }
            }
        }

        self.tracer.add_event(event)
    }

    /// Set the status of the current span
    pub fn set_span_status(&self, status: SpanStatus) -> Result<()> {
        // Check capability if a checker is available and enforcement is enabled
        if self.enforce_capabilities {
            if let Some(checker) = &self.capability_checker {
                let has_capability =
                    checker.check_capability(&self.plugin_id, ObservabilityCapability::Tracing)?;
                if !has_capability {
                    return Err(ObservabilityError::CapabilityError(
                        "Missing tracing capability".to_string(),
                    ));
                }
            }
        }

        self.tracer.set_status(status)
    }
}

/// Manager for plugin observability
pub struct PluginObservabilityManager {
    /// Logger
    logger: Arc<SimpleLogger>,

    /// Tracer
    tracer: Arc<NoopTracer>,

    /// Metrics registry
    metrics_registry: Arc<dyn MetricsRegistry>,

    /// Capability checker (optional)
    capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,

    /// Whether to enforce capabilities
    enforce_capabilities: bool,

    /// Plugin observability instances
    plugins: dashmap::DashMap<String, PluginObservability>,
}

impl PluginObservabilityManager {
    /// Create a new plugin observability manager
    pub fn new(
        logger: Arc<SimpleLogger>,
        tracer: Arc<NoopTracer>,
        metrics_registry: Arc<dyn MetricsRegistry>,
        capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,
        enforce_capabilities: bool,
    ) -> Self {
        Self {
            logger,
            tracer,
            metrics_registry,
            capability_checker,
            enforce_capabilities,
            plugins: dashmap::DashMap::new(),
        }
    }

    /// Get or create plugin observability for a plugin
    pub fn get_or_create(&self, plugin_id: &str) -> Result<PluginObservability> {
        if let Some(obs) = self.plugins.get(plugin_id) {
            return Ok(obs.clone());
        }

        // Validate plugin ID
        if plugin_id.is_empty() {
            return Err(ObservabilityError::InvalidPluginId(
                "Plugin ID cannot be empty".to_string(),
            ));
        }

        // Create new plugin observability
        let obs = PluginObservability::new(
            plugin_id.to_string(),
            self.logger.clone(),
            self.tracer.clone(),
            self.metrics_registry.clone(),
            self.capability_checker.clone(),
            self.enforce_capabilities,
        );

        // Store and return
        self.plugins.insert(plugin_id.to_string(), obs.clone());
        Ok(obs)
    }

    /// Remove plugin observability for a plugin
    pub fn remove(&self, plugin_id: &str) -> Result<()> {
        self.plugins.remove(plugin_id);
        Ok(())
    }

    /// Check if a plugin exists
    pub fn exists(&self, plugin_id: &str) -> bool {
        self.plugins.contains_key(plugin_id)
    }

    /// Get all plugin IDs
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.iter().map(|r| r.key().clone()).collect()
    }

    /// Get the current number of plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoggingConfig;
    use crate::metrics::NoopMetricsRegistry;

    #[test]
    fn test_plugin_observability() {
        let logger = Arc::new(SimpleLogger::new(&LoggingConfig::default()));
        let tracer = Arc::new(NoopTracer::new());
        let metrics = Arc::new(NoopMetricsRegistry::new());

        let obs = PluginObservability::new(
            "test_plugin".to_string(),
            logger,
            tracer,
            metrics,
            None,
            false,
        );

        assert_eq!(obs.plugin_id(), "test_plugin");

        // Test logging
        assert!(obs.info("Test message").is_ok());

        // Test metrics
        let counter = obs
            .counter("test_counter", "Test counter", HashMap::new())
            .unwrap();
        assert!(counter.increment(1).is_ok());

        // Test tracing
        let ctx = obs.create_context();
        ctx.with_current(|| {
            let span = obs.start_span("test_span").unwrap();
            assert_eq!(span.name, "test_span");
            assert!(span.attributes.contains_key("plugin_id"));
            assert_eq!(
                span.attributes.get("plugin_id").unwrap().as_str().unwrap(),
                "test_plugin"
            );
        });
    }

    #[test]
    fn test_plugin_manager() {
        let logger = Arc::new(SimpleLogger::new(&LoggingConfig::default()));
        let tracer = Arc::new(NoopTracer::new());
        let metrics = Arc::new(NoopMetricsRegistry::new());

        let manager = PluginObservabilityManager::new(logger, tracer, metrics, None, false);

        // Get or create a plugin
        let obs1 = manager.get_or_create("plugin1").unwrap();
        assert_eq!(obs1.plugin_id(), "plugin1");

        // Get the same plugin again
        let obs1_again = manager.get_or_create("plugin1").unwrap();
        assert_eq!(obs1_again.plugin_id(), "plugin1");

        // Get a different plugin
        let obs2 = manager.get_or_create("plugin2").unwrap();
        assert_eq!(obs2.plugin_id(), "plugin2");

        // Check plugin count
        assert_eq!(manager.plugin_count(), 2);

        // Check plugin IDs
        let ids = manager.plugin_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"plugin1".to_string()));
        assert!(ids.contains(&"plugin2".to_string()));

        // Remove a plugin
        manager.remove("plugin1").unwrap();
        assert_eq!(manager.plugin_count(), 1);
        assert!(!manager.exists("plugin1"));
        assert!(manager.exists("plugin2"));
    }
}
