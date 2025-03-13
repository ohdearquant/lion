//! Interface to the Lion observability component
//!
//! This module provides functions to interact with the Lion observability system,
//! which is responsible for logging, metrics, and tracing.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Get system logs with optional filtering
pub fn get_logs(level: Option<&str>, component: Option<&str>) -> Result<Vec<LogEntry>> {
    // In a real implementation, this would call into lion_observability::logging
    #[cfg(feature = "observability-integration")]
    {
        use lion_observability::logging::LogStore;

        let store = LogStore::global();
        let mut filter = LogFilter::new();

        if let Some(lvl) = level {
            filter.level(lvl);
        }

        if let Some(comp) = component {
            filter.component(comp);
        }

        let logs = store.get_logs(filter)?;

        let mut result = Vec::new();
        for log in logs {
            result.push(LogEntry {
                timestamp: log.timestamp,
                level: log.level,
                component: log.component,
                message: log.message,
                plugin_id: log.plugin_id,
                correlation_id: log.correlation_id,
            });
        }

        Ok(result)
    }

    #[cfg(not(feature = "observability-integration"))]
    {
        // Placeholder implementation
        println!("Getting system logs");

        if let Some(lvl) = level {
            println!("Filtering by level: {}", lvl);
        }

        if let Some(comp) = component {
            println!("Filtering by component: {}", comp);
        }

        // Mock log entries
        let mut logs = vec![
            LogEntry {
                timestamp: "2025-03-13T14:45:12Z".to_string(),
                level: "INFO".to_string(),
                component: "system".to_string(),
                message: "Lion microkernel starting".to_string(),
                plugin_id: None,
                correlation_id: None,
            },
            LogEntry {
                timestamp: "2025-03-13T14:45:13Z".to_string(),
                level: "INFO".to_string(),
                component: "capability".to_string(),
                message: "Capability store initialized".to_string(),
                plugin_id: None,
                correlation_id: None,
            },
            LogEntry {
                timestamp: "2025-03-13T14:45:14Z".to_string(),
                level: "INFO".to_string(),
                component: "plugin".to_string(),
                message: "Plugin 'calculator' loaded".to_string(),
                plugin_id: Some("123e4567-e89b-12d3-a456-426614174000".to_string()),
                correlation_id: None,
            },
            LogEntry {
                timestamp: "2025-03-13T14:45:15Z".to_string(),
                level: "WARN".to_string(),
                component: "network".to_string(),
                message: "Rate limiting applied to network requests".to_string(),
                plugin_id: None,
                correlation_id: None,
            },
            LogEntry {
                timestamp: "2025-03-13T14:45:16Z".to_string(),
                level: "INFO".to_string(),
                component: "workflow".to_string(),
                message: "Workflow 'data-processing' started".to_string(),
                plugin_id: None,
                correlation_id: Some("workflow-123".to_string()),
            },
        ];

        // Apply level filter if provided
        if let Some(lvl) = level {
            logs.retain(|log| log.level.to_lowercase() == lvl.to_lowercase());
        }

        // Apply component filter if provided
        if let Some(comp) = component {
            logs.retain(|log| log.component.to_lowercase() == comp.to_lowercase());
        }

        Ok(logs)
    }
}

/// Get system metrics
pub fn get_metrics() -> Result<HashMap<String, MetricValue>> {
    // In a real implementation, this would call into lion_observability::metrics
    #[cfg(feature = "observability-integration")]
    {
        use lion_observability::metrics::MetricsCollector;

        let collector = MetricsCollector::global();
        let metrics = collector.get_all_metrics()?;

        let mut result = HashMap::new();
        for (name, value) in metrics {
            result.insert(
                name,
                match value {
                    Metric::Counter(v) => MetricValue::Counter(v),
                    Metric::Gauge(v) => MetricValue::Gauge(v),
                    Metric::Histogram(v) => MetricValue::Histogram(v),
                },
            );
        }

        Ok(result)
    }

    #[cfg(not(feature = "observability-integration"))]
    {
        // Placeholder implementation
        println!("Getting system metrics");

        // Mock metrics
        let mut metrics = HashMap::new();

        // System metrics
        metrics.insert(
            "system.memory.used_mb".to_string(),
            MetricValue::Gauge(42.5),
        );
        metrics.insert(
            "system.cpu.usage_percent".to_string(),
            MetricValue::Gauge(2.3),
        );
        metrics.insert("system.plugins.loaded".to_string(), MetricValue::Gauge(2.0));
        metrics.insert(
            "system.workflows.active".to_string(),
            MetricValue::Gauge(1.0),
        );

        // Plugin metrics
        metrics.insert(
            "plugin.calculator.calls".to_string(),
            MetricValue::Counter(15.0),
        );
        metrics.insert(
            "plugin.calculator.errors".to_string(),
            MetricValue::Counter(0.0),
        );
        metrics.insert(
            "plugin.calculator.execution_time_ms".to_string(),
            MetricValue::Histogram(vec![12.0, 15.0, 10.0, 11.0, 13.0]),
        );

        // Network metrics
        metrics.insert("network.requests".to_string(), MetricValue::Counter(25.0));
        metrics.insert(
            "network.bytes_sent".to_string(),
            MetricValue::Counter(1024.0),
        );
        metrics.insert(
            "network.bytes_received".to_string(),
            MetricValue::Counter(2048.0),
        );

        Ok(metrics)
    }
}

/// Enable or disable tracing
pub fn set_tracing_enabled(enabled: bool) -> Result<()> {
    // In a real implementation, this would call into lion_observability::tracing_system
    #[cfg(feature = "observability-integration")]
    {
        use lion_observability::tracing_system::TracingSystem;

        let tracing = TracingSystem::global();

        if enabled {
            tracing.enable()?;
        } else {
            tracing.disable()?;
        }
    }

    #[cfg(not(feature = "observability-integration"))]
    {
        // Placeholder implementation
        if enabled {
            println!("Enabling distributed tracing");
        } else {
            println!("Disabling distributed tracing");
        }
    }

    Ok(())
}

/// Get trace for a specific correlation ID
pub fn get_trace(correlation_id: &str) -> Result<Vec<TraceSpan>> {
    // In a real implementation, this would call into lion_observability::tracing_system
    #[cfg(feature = "observability-integration")]
    {
        use lion_observability::tracing_system::TracingSystem;

        let tracing = TracingSystem::global();
        let spans = tracing.get_trace(correlation_id)?;

        let mut result = Vec::new();
        for span in spans {
            result.push(TraceSpan {
                id: span.id,
                parent_id: span.parent_id,
                name: span.name,
                start_time: span.start_time,
                end_time: span.end_time,
                attributes: span.attributes,
            });
        }

        Ok(result)
    }

    #[cfg(not(feature = "observability-integration"))]
    {
        // Placeholder implementation
        println!("Getting trace for correlation ID: {}", correlation_id);

        // Mock trace spans
        Ok(vec![
            TraceSpan {
                id: "span1".to_string(),
                parent_id: None,
                name: "workflow.execute".to_string(),
                start_time: "2025-03-13T14:45:16.000Z".to_string(),
                end_time: Some("2025-03-13T14:45:16.500Z".to_string()),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("workflow.id".to_string(), "data-processing".to_string());
                    attrs
                },
            },
            TraceSpan {
                id: "span2".to_string(),
                parent_id: Some("span1".to_string()),
                name: "plugin.call".to_string(),
                start_time: "2025-03-13T14:45:16.100Z".to_string(),
                end_time: Some("2025-03-13T14:45:16.300Z".to_string()),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("plugin.id".to_string(), "calculator".to_string());
                    attrs.insert("function".to_string(), "calculate".to_string());
                    attrs
                },
            },
            TraceSpan {
                id: "span3".to_string(),
                parent_id: Some("span1".to_string()),
                name: "plugin.call".to_string(),
                start_time: "2025-03-13T14:45:16.350Z".to_string(),
                end_time: Some("2025-03-13T14:45:16.450Z".to_string()),
                attributes: {
                    let mut attrs = HashMap::new();
                    attrs.insert("plugin.id".to_string(), "text-processor".to_string());
                    attrs.insert("function".to_string(), "process".to_string());
                    attrs
                },
            },
        ])
    }
}

/// A log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub component: String,
    pub message: String,
    pub plugin_id: Option<String>,
    pub correlation_id: Option<String>,
}

/// A metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram(Vec<f64>),
}

/// A trace span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub attributes: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_logs() {
        let result = get_logs(None, None);
        assert!(result.is_ok());

        let logs = result.unwrap();
        assert!(!logs.is_empty());

        // Test with level filter
        let result = get_logs(Some("INFO"), None);
        assert!(result.is_ok());

        let logs = result.unwrap();
        for log in &logs {
            assert_eq!(log.level, "INFO");
        }

        // Test with component filter
        let result = get_logs(None, Some("system"));
        assert!(result.is_ok());

        let logs = result.unwrap();
        for log in &logs {
            assert_eq!(log.component, "system");
        }
    }

    #[test]
    fn test_get_metrics() {
        let result = get_metrics();
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(!metrics.is_empty());

        // Check for specific metrics
        assert!(metrics.contains_key("system.memory.used_mb"));
        assert!(metrics.contains_key("system.cpu.usage_percent"));
    }

    #[test]
    fn test_set_tracing_enabled() {
        let result = set_tracing_enabled(true);
        assert!(result.is_ok());

        let result = set_tracing_enabled(false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_trace() {
        let correlation_id = "workflow-123";
        let result = get_trace(correlation_id);
        assert!(result.is_ok());

        let spans = result.unwrap();
        assert!(!spans.is_empty());

        // Check span hierarchy
        let root_spans: Vec<_> = spans
            .iter()
            .filter(|span| span.parent_id.is_none())
            .collect();
        assert!(!root_spans.is_empty());

        let child_spans: Vec<_> = spans
            .iter()
            .filter(|span| span.parent_id.is_some())
            .collect();
        for child in child_spans {
            let parent_id = child.parent_id.as_ref().unwrap();
            assert!(spans.iter().any(|span| &span.id == parent_id));
        }
    }
}
