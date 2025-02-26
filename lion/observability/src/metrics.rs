//! Metrics integration for the Lion runtime.
//!
//! This module provides integration with the metrics ecosystem
//! for collecting and exporting metrics.

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use core::error::{Result, ObservabilityError};

/// Configuration for metrics.
#[derive(Clone, Debug)]
pub struct MetricsConfig {
    /// Service name for metrics.
    pub service_name: String,
    
    /// Whether to enable Prometheus metrics.
    pub enable_prometheus: bool,
    
    /// Prometheus listen address.
    pub prometheus_addr: Option<SocketAddr>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            service_name: "lion".to_string(),
            enable_prometheus: false,
            prometheus_addr: None,
        }
    }
}

/// Handle for metrics resources.
pub struct MetricsHandle {
    /// Optional handle for the Prometheus exporter.
    _prometheus_handle: Option<Arc<metrics_exporter_prometheus::PrometheusHandle>>,
}

/// Initialize metrics.
pub fn init_metrics(config: MetricsConfig) -> Result<MetricsHandle> {
    // Initialize Prometheus if enabled
    let prometheus_handle = if config.enable_prometheus {
        if let Some(addr) = config.prometheus_addr {
            // Create a Prometheus recorder
            let (recorder, exporter) = metrics_exporter_prometheus::PrometheusBuilder::new()
                .with_namespace(config.service_name)
                .build()
                .map_err(|e| ObservabilityError::MetricsInitFailed(
                    format!("Failed to build Prometheus recorder: {}", e)
                ))?;
            
            // Install the recorder
            metrics::set_boxed_recorder(Box::new(recorder))
                .map_err(|e| ObservabilityError::MetricsInitFailed(
                    format!("Failed to install metrics recorder: {}", e)
                ))?;
            
            // Start a HTTP server to expose metrics
            let handle = exporter.handle();
            thread::spawn(move || {
                let addr = addr;
                let exporter = exporter;
                if let Err(e) = exporter.install() {
                    eprintln!("Failed to start Prometheus exporter: {}", e);
                }
            });
            
            Some(handle)
        } else {
            None
        }
    } else {
        None
    };
    
    Ok(MetricsHandle {
        _prometheus_handle: prometheus_handle,
    })
}

/// Record plugin function call.
pub fn record_plugin_call(
    plugin_id: &core::types::PluginId,
    function: &str,
    duration_ms: u64,
    success: bool,
) {
    let plugin_id_str = plugin_id.to_string();
    
    // Record call count
    metrics::counter!("lion.plugin.calls", 1, 
        "plugin_id" => plugin_id_str.clone(),
        "function" => function.to_string(),
        "success" => success.to_string()
    );
    
    // Record call duration
    metrics::histogram!("lion.plugin.call_duration_ms", duration_ms as f64,
        "plugin_id" => plugin_id_str.clone(),
        "function" => function.to_string()
    );
}

/// Record memory usage.
pub fn record_memory_usage(
    plugin_id: &core::types::PluginId,
    memory_bytes: usize,
) {
    let plugin_id_str = plugin_id.to_string();
    
    // Record memory usage
    metrics::gauge!("lion.plugin.memory_bytes", memory_bytes as f64,
        "plugin_id" => plugin_id_str
    );
}

/// Record workflow execution.
pub fn record_workflow_execution(
    workflow_id: &crate::workflow::WorkflowId,
    execution_id: &crate::workflow::ExecutionId,
    duration_ms: u64,
    success: bool,
) {
    let workflow_id_str = workflow_id.to_string();
    let execution_id_str = execution_id.to_string();
    
    // Record execution count
    metrics::counter!("lion.workflow.executions", 1,
        "workflow_id" => workflow_id_str.clone(),
        "execution_id" => execution_id_str.clone(),
        "success" => success.to_string()
    );
    
    // Record execution duration
    metrics::histogram!("lion.workflow.execution_duration_ms", duration_ms as f64,
        "workflow_id" => workflow_id_str,
        "execution_id" => execution_id_str
    );
}