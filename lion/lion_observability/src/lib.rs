//! # Lion Observability
//!
//! This crate provides observability infrastructure for the Lion microkernel,
//! including tracing, logging, and metrics collection. All observability features
//! are designed with Lion's capability-based security model in mind.
//!
//! ## Features
//!
//! - **Structured Logging**: JSON-structured logs with context propagation
//! - **Distributed Tracing**: OpenTelemetry-compatible distributed tracing
//! - **Metrics Collection**: Lightweight performance metrics with Prometheus export
//! - **Security Integration**: Capability-based access to observability features
//!
//! ## Design Principles
//!
//! - **Security**: Observability features require explicit capabilities
//! - **Performance**: Low overhead (target 5-10%)
//! - **Modularity**: Clean separation of logging, metrics, and tracing
//! - **Integration**: Works with Lion's existing capability and plugin systems

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

use std::sync::Arc;

// Re-export tracing for convenience
pub use tracing;

// Module structure
pub mod capability;
pub mod config;
pub mod context;
pub mod error;
pub mod logging;
pub mod metrics;
pub mod plugin;
pub mod tracing_system;

// Public exports
pub use capability::{ObservabilityCapability, ObservabilityCapabilityChecker};
pub use config::{LoggingConfig, MetricsConfig, ObservabilityConfig, TracingConfig};
pub use context::{Context, SpanContext};
pub use error::ObservabilityError;
pub use logging::{LogEvent, Logger, LoggerBase};
pub use metrics::{Counter, Gauge, Histogram, Metric, MetricsRegistry};
pub use plugin::{PluginObservability, PluginObservabilityManager};
pub use tracing_system::{Span, Tracer, TracerBase, TracingEvent};

/// Result type for observability operations
pub type Result<T> = std::result::Result<T, ObservabilityError>;

/// Main entry point for the observability system
///
/// This struct provides access to all observability features
/// and ensures they're properly initialized.
#[derive(Clone)]
pub struct Observability {
    config: Arc<ObservabilityConfig>,
    logger: Arc<logging::SimpleLogger>,
    tracer: Arc<tracing_system::NoopTracer>,
    metrics_registry: Arc<dyn MetricsRegistry>,
    capability_checker: Option<Arc<dyn ObservabilityCapabilityChecker>>,
}

impl Observability {
    /// Create a new Observability instance with the provided configuration
    pub fn new(config: ObservabilityConfig) -> Result<Self> {
        // Create concrete instances directly instead of using the factory functions
        let logger = if !config.logging.enabled {
            Arc::new(logging::SimpleLogger::new(&LoggingConfig::default()))
        } else {
            Arc::new(logging::SimpleLogger::new(&config.logging))
        };

        let tracer = if !config.tracing.enabled {
            Arc::new(tracing_system::NoopTracer::new())
        } else {
            // For simplicity, we'll just use NoopTracer for now
            Arc::new(tracing_system::NoopTracer::new())
        };

        let metrics_registry: Arc<dyn MetricsRegistry> = if !config.metrics.enabled {
            Arc::new(metrics::NoopMetricsRegistry::new())
        } else {
            let prometheus_registry = metrics::PrometheusMetricsRegistry::new(&config.metrics)?;
            Arc::new(prometheus_registry)
        };

        Ok(Self {
            config: Arc::new(config),
            logger,
            tracer,
            metrics_registry,
            capability_checker: None,
        })
    }

    /// Set the capability checker for enforcing security policies
    pub fn with_capability_checker<C: ObservabilityCapabilityChecker + 'static>(
        mut self,
        checker: C,
    ) -> Self {
        self.capability_checker = Some(Arc::new(checker));
        self
    }

    /// Get the logger instance
    pub fn logger(&self) -> Arc<logging::SimpleLogger> {
        self.logger.clone()
    }

    /// Get the tracer instance
    pub fn tracer(&self) -> Arc<tracing_system::NoopTracer> {
        self.tracer.clone()
    }

    /// Get the metrics registry
    pub fn metrics_registry(&self) -> Arc<dyn MetricsRegistry> {
        self.metrics_registry.clone()
    }

    /// Create an observability context for a specific plugin
    pub fn create_plugin_observability(&self, plugin_id: &str) -> PluginObservability {
        PluginObservability::new(
            plugin_id.to_string(),
            self.logger.clone(),
            self.tracer.clone(),
            self.metrics_registry.clone(),
            self.capability_checker.clone(),
        )
    }

    /// Shutdown the observability system, flushing any pending data
    pub fn shutdown(&self) -> Result<()> {
        self.logger.shutdown()?;
        self.tracer.shutdown()?;
        self.metrics_registry.shutdown()?;
        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &ObservabilityConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_observability() {
        let config = ObservabilityConfig::default();
        let obs = Observability::new(config).expect("Failed to create observability");

        assert!(obs.logger().as_ref().name().contains("default"));
        assert!(obs.tracer().as_ref().name().contains("default"));
        assert!(obs.metrics_registry().as_ref().name().contains("default"));
    }
}
