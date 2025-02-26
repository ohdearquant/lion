//! Lion Observability - Telemetry integration
//!
//! This crate provides observability features for the Lion runtime,
//! including tracing, metrics, and structured logging.

mod tracing;
mod metrics;

pub use self::tracing::{TracingConfig, init_tracing};
pub use self::metrics::{MetricsConfig, init_metrics};

use std::sync::Arc;

use core::error::Result;

/// Initialize observability for the Lion runtime.
pub fn init(tracing_config: Option<TracingConfig>, metrics_config: Option<MetricsConfig>) -> Result<ObservabilityHandle> {
    // Initialize tracing if configured
    let _tracing_guard = if let Some(config) = tracing_config {
        Some(init_tracing(config)?)
    } else {
        None
    };
    
    // Initialize metrics if configured
    let _metrics_handle = if let Some(config) = metrics_config {
        Some(init_metrics(config)?)
    } else {
        None
    };
    
    Ok(ObservabilityHandle {
        _tracing_guard,
        _metrics_handle,
    })
}

/// Handle for observability resources.
pub struct ObservabilityHandle {
    /// Guard for tracing resources.
    _tracing_guard: Option<tracing::TracingGuard>,
    
    /// Handle for metrics resources.
    _metrics_handle: Option<metrics::MetricsHandle>,
}

impl ObservabilityHandle {
    /// Shut down observability.
    pub fn shutdown(&self) -> Result<()> {
        // Shutdown will be handled on drop
        Ok(())
    }
}