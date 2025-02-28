//! Configuration for the observability system

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for the logging subsystem
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Whether to enable logging
    pub enabled: bool,

    /// The base log level
    pub level: String,

    /// Whether to use structured (JSON) logging
    pub structured: bool,

    /// Optional file path for logs
    pub file_path: Option<PathBuf>,

    /// Maximum log file size in bytes before rotation
    pub max_file_size: usize,

    /// Maximum number of rotated log files to keep
    pub max_files: usize,

    /// Whether to log to stdout
    pub log_to_stdout: bool,

    /// Whether to log to stderr
    pub log_to_stderr: bool,

    /// Whether to include plugin ID in logs
    pub include_plugin_id: bool,

    /// Whether to include trace context in logs
    pub include_trace_context: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
            structured: true,
            file_path: None,
            max_file_size: 50 * 1024 * 1024, // 50 MiB
            max_files: 5,
            log_to_stdout: true,
            log_to_stderr: false,
            include_plugin_id: true,
            include_trace_context: true,
        }
    }
}

/// Configuration for the tracing subsystem
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Whether to enable tracing
    pub enabled: bool,

    /// Sampling rate (0.0-1.0) for traces
    pub sampling_rate: f64,

    /// Whether to enable OpenTelemetry export
    pub export_enabled: bool,

    /// OpenTelemetry collector endpoint
    pub collector_endpoint: Option<String>,

    /// Trace context propagation mode
    pub propagation: TracePropagation,

    /// Maximum events per span
    pub max_events_per_span: usize,

    /// Maximum attributes per span
    pub max_attributes_per_span: usize,

    /// Maximum links per span
    pub max_links_per_span: usize,

    /// Batch export size
    pub batch_size: usize,

    /// Batch export timeout
    pub batch_timeout: Duration,

    /// Whether to include plugin ID in traces
    pub include_plugin_id: bool,
}

/// Trace context propagation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracePropagation {
    /// W3C TraceContext
    W3C,
    /// B3 single header
    B3Single,
    /// B3 multi header
    B3Multi,
    /// Jaeger propagation
    Jaeger,
    /// Custom propagation
    Custom,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sampling_rate: 0.01, // 1% sampling
            export_enabled: false,
            collector_endpoint: None,
            propagation: TracePropagation::W3C,
            max_events_per_span: 128,
            max_attributes_per_span: 32,
            max_links_per_span: 32,
            batch_size: 512,
            batch_timeout: Duration::from_secs(5),
            include_plugin_id: true,
        }
    }
}

/// Configuration for the metrics subsystem
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Whether to enable metrics
    pub enabled: bool,

    /// Whether to enable Prometheus export
    pub prometheus_enabled: bool,

    /// Prometheus export endpoint
    pub prometheus_endpoint: String,

    /// Whether to enable OpenTelemetry export
    pub otlp_enabled: bool,

    /// OpenTelemetry metrics endpoint
    pub otlp_endpoint: Option<String>,

    /// Default labels to add to all metrics
    pub default_labels: Vec<(String, String)>,

    /// Whether to include plugin ID in metrics
    pub include_plugin_id: bool,

    /// Metrics export interval
    pub export_interval: Duration,

    /// Whether to record histogram buckets
    pub histogram_enabled: bool,

    /// Default histogram buckets
    pub default_buckets: Vec<f64>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prometheus_enabled: true,
            prometheus_endpoint: "0.0.0.0:9090".to_string(),
            otlp_enabled: false,
            otlp_endpoint: None,
            default_labels: vec![("service".to_string(), "lion".to_string())],
            include_plugin_id: true,
            export_interval: Duration::from_secs(15),
            histogram_enabled: true,
            default_buckets: vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
        }
    }
}

/// Master configuration for the observability system
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Logging configuration
    pub logging: LoggingConfig,

    /// Tracing configuration
    pub tracing: TracingConfig,

    /// Metrics configuration
    pub metrics: MetricsConfig,

    /// Rate limiting for logging (events per second)
    pub log_rate_limit: Option<u32>,

    /// Whether to enforce capability checks
    pub enforce_capabilities: bool,

    /// Whether to buffer events when capabilities are missing
    pub buffer_blocked_events: bool,

    /// Maximum buffer size for blocked events
    pub max_buffer_size: usize,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            logging: LoggingConfig::default(),
            tracing: TracingConfig::default(),
            metrics: MetricsConfig::default(),
            log_rate_limit: Some(10000), // 10k logs/sec
            enforce_capabilities: true,
            buffer_blocked_events: false,
            max_buffer_size: 1024,
        }
    }
}

impl ObservabilityConfig {
    /// Create a minimal configuration for testing
    pub fn minimal_for_testing() -> Self {
        let mut config = Self::default();
        config.logging.file_path = None;
        config.logging.log_to_stdout = false;
        config.tracing.enabled = false;
        config.metrics.enabled = false;
        config.enforce_capabilities = false;
        config
    }

    /// Create a development configuration
    pub fn development() -> Self {
        let mut config = Self::default();
        config.logging.level = "debug".to_string();
        config.logging.structured = false;
        config.tracing.sampling_rate = 1.0; // Sample all traces
        config.enforce_capabilities = false;
        config
    }

    /// Create a production configuration
    pub fn production() -> Self {
        let mut config = Self::default();
        config.logging.level = "info".to_string();
        config.logging.structured = true;
        config.logging.log_to_stdout = false;
        config.logging.file_path = Some(PathBuf::from("/var/log/lion/lion.log"));
        config.tracing.sampling_rate = 0.01; // 1% sampling
        config.metrics.prometheus_enabled = true;
        config.enforce_capabilities = true;
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert!(config.logging.enabled);
        assert!(config.tracing.enabled);
        assert!(config.metrics.enabled);
        assert!(config.enforce_capabilities);
    }

    #[test]
    fn test_minimal_config() {
        let config = ObservabilityConfig::minimal_for_testing();
        assert!(config.logging.enabled);
        assert!(!config.tracing.enabled);
        assert!(!config.metrics.enabled);
        assert!(!config.enforce_capabilities);
    }

    #[test]
    fn test_dev_vs_prod() {
        let dev = ObservabilityConfig::development();
        let prod = ObservabilityConfig::production();

        assert_eq!(dev.logging.level, "debug");
        assert_eq!(prod.logging.level, "info");

        assert!(!dev.logging.structured);
        assert!(prod.logging.structured);

        assert!(!dev.enforce_capabilities);
        assert!(prod.enforce_capabilities);
    }
}
