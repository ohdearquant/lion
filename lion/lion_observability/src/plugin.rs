//! Plugin observability management
//!
//! This module provides observability functionality for plugins
//! with capability-based access control and context propagation.

use std::collections::HashMap;
use std::sync::Arc;

use crate::capability::{LogLevel, ObservabilityCapabilityChecker};
use crate::context::{Context, SpanContext};
use crate::error::ObservabilityError;
use crate::logging::{LogEvent, LoggerBase};
use crate::metrics::{Counter, Gauge, Histogram, MetricsRegistry};
use crate::tracing_system::{Span, SpanStatus, Tracer, TracerBase, TracingEvent};
use crate::Result;

/// Observability for a specific plugin
#[derive(Clone)]
pub struct PluginObservability {
    /// Plugin ID
    plugin_id: String,

    /// Logger
    logger: Arc<dyn LoggerBase>,

    /// Tracer
    tracer: Arc<dyn TracerBase>,

    /// Metrics registry
    metrics_registry: Arc<dyn MetricsRegistry>,

    /// Capability checker (optional)
    capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,
}

impl PluginObservability {
    /// Create a new plugin observability instance
    pub fn new(
        plugin_id: String,
        logger: Arc<dyn LoggerBase>,
        tracer: Arc<dyn TracerBase>,
        metrics_registry: Arc<dyn MetricsRegistry>,
        capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,
    ) -> Self {
        Self {
            plugin_id,
            logger,
            tracer,
            metrics_registry,
            capability_checker,
        }
    }

    /// Start a span with the plugin context
    pub fn start_span(&self, name: impl Into<String>) -> Result<Span> {
        // Create the span
        let mut span = self.tracer.create_span(name)?;

        // Add plugin ID as an attribute
        span = span.with_attribute("plugin_id", self.plugin_id.clone())?;

        Ok(span)
    }

    /// Run a function within a span
    pub fn with_span<F, R>(&self, name: impl Into<String>, f: F) -> Result<R>
    where
        F: FnOnce() -> R,
    {
        // Create a context with the plugin ID
        let ctx = Context::new().with_plugin_id(self.plugin_id.clone());

        // Run with this context
        ctx.with_current(|| self.tracer.with_span(name, f))
    }

    /// Create or get a counter
    pub fn counter(
        &self,
        name: &str,
        description: &str,
        labels: HashMap<String, String>,
    ) -> Result<Arc<dyn Counter>> {
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
        // Add plugin ID to labels
        let mut labels = labels;
        labels.insert("plugin_id".to_string(), self.plugin_id.clone());

        // Create the histogram
        self.metrics_registry.histogram(name, description, labels)
    }

    /// Log a message at the specified level
    pub fn log(&self, level: LogLevel, message: impl Into<String>) -> Result<()> {
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
        Context::new().with_plugin_id(self.plugin_id.clone())
    }

    /// Create a context with the plugin ID and span
    pub fn create_context_with_span(&self, span_context: SpanContext) -> Context {
        Context::new()
            .with_plugin_id(self.plugin_id.clone())
            .with_span_context(span_context)
    }

    /// Add an event to the current span
    pub fn add_span_event(&self, event: TracingEvent) -> Result<()> {
        self.tracer.add_event(event)
    }

    /// Set the status of the current span
    pub fn set_span_status(&self, status: SpanStatus) -> Result<()> {
        self.tracer.set_status(status)
    }
}

/// Manager for plugin observability
pub struct PluginObservabilityManager {
    /// Logger
    logger: Arc<dyn LoggerBase>,

    /// Tracer
    tracer: Arc<dyn TracerBase>,

    /// Metrics registry
    metrics_registry: Arc<dyn MetricsRegistry>,

    /// Capability checker (optional)
    capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,

    /// Plugin observability instances
    plugins: dashmap::DashMap<String, PluginObservability>,
}

impl PluginObservabilityManager {
    /// Create a new plugin observability manager
    pub fn new(
        logger: Arc<dyn LoggerBase>,
        tracer: Arc<dyn TracerBase>,
        metrics_registry: Arc<dyn MetricsRegistry>,
        capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,
    ) -> Self {
        Self {
            logger,
            tracer,
            metrics_registry,
            capability_checker,
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
    use crate::logging::NoopLogger;
    use crate::metrics::NoopMetricsRegistry;
    use crate::tracing_system::NoopTracer;

    #[test]
    fn test_plugin_observability() {
        let logger = Arc::new(NoopLogger::new());
        let tracer = Arc::new(NoopTracer::new());
        let metrics = Arc::new(NoopMetricsRegistry::new());

        let obs =
            PluginObservability::new("test_plugin".to_string(), logger, tracer, metrics, None);

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
        let logger = Arc::new(NoopLogger::new());
        let tracer = Arc::new(NoopTracer::new());
        let metrics = Arc::new(NoopMetricsRegistry::new());

        let manager = PluginObservabilityManager::new(logger, tracer, metrics, None);

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
