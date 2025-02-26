//! Tracing integration for the Lion runtime.
//!
//! This module provides integration with the tracing ecosystem
//! for structured logging and distributed tracing.

use std::io;
use std::path::PathBuf;

use core::error::{Result, ObservabilityError};
use tracing_subscriber::prelude::*;

/// Configuration for tracing.
#[derive(Clone, Debug)]
pub struct TracingConfig {
    /// Service name for tracing.
    pub service_name: String,
    
    /// Log level.
    pub log_level: String,
    
    /// Whether to enable file logging.
    pub enable_file_logging: bool,
    
    /// Directory for log files.
    pub log_directory: Option<PathBuf>,
    
    /// Whether to enable console logging.
    pub enable_console_logging: bool,
    
    /// Whether to enable JSON formatting.
    pub enable_json_format: bool,
    
    /// Whether to enable Jaeger tracing.
    pub enable_jaeger: bool,
    
    /// Jaeger endpoint.
    pub jaeger_endpoint: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "lion".to_string(),
            log_level: "info".to_string(),
            enable_file_logging: false,
            log_directory: None,
            enable_console_logging: true,
            enable_json_format: false,
            enable_jaeger: false,
            jaeger_endpoint: None,
        }
    }
}

/// Handle for tracing resources.
pub struct TracingGuard {
    /// Optional guard for file logging.
    _file_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    
    /// Optional guard for Jaeger tracing.
    _jaeger_pipeline: Option<opentelemetry::sdk::trace::Tracer>,
}

/// Initialize tracing.
pub fn init_tracing(config: TracingConfig) -> Result<TracingGuard> {
    // Build subscriber
    let mut layers = Vec::new();
    
    // Create the filter
    let filter = tracing_subscriber::EnvFilter::new(&config.log_level);
    
    // Create the console layer if enabled
    if config.enable_console_logging {
        if config.enable_json_format {
            let console_layer = tracing_subscriber::fmt::layer()
                .json()
                .with_writer(io::stdout);
            layers.push(console_layer.boxed());
        } else {
            let console_layer = tracing_subscriber::fmt::layer()
                .with_writer(io::stdout);
            layers.push(console_layer.boxed());
        }
    }
    
    // Create the file layer if enabled
    let file_guard = if config.enable_file_logging {
        if let Some(log_dir) = &config.log_directory {
            // Create the directory if it doesn't exist
            std::fs::create_dir_all(log_dir)
                .map_err(|e| ObservabilityError::TracingInitFailed(
                    format!("Failed to create log directory: {}", e)
                ))?;
            
            // Create a rolling file appender
            let file_appender = tracing_appender::rolling::daily(
                log_dir,
                format!("{}.log", config.service_name),
            );
            
            // Create a non-blocking writer
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            
            // Create the file layer
            if config.enable_json_format {
                let file_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(non_blocking);
                layers.push(file_layer.boxed());
            } else {
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(non_blocking);
                layers.push(file_layer.boxed());
            }
            
            Some(guard)
        } else {
            None
        }
    } else {
        None
    };
    
    // Create the Jaeger layer if enabled
    let jaeger_pipeline = if config.enable_jaeger {
        // Create the Jaeger exporter
        let mut jaeger_pipeline_builder = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name(&config.service_name);
        
        // Add endpoint if provided
        if let Some(endpoint) = &config.jaeger_endpoint {
            jaeger_pipeline_builder = jaeger_pipeline_builder.with_endpoint(endpoint);
        }
        
        // Install the pipeline
        let tracer = jaeger_pipeline_builder
            .install_batch(opentelemetry::runtime::Tokio)
            .map_err(|e| ObservabilityError::TracingInitFailed(
                format!("Failed to install Jaeger pipeline: {}", e)
            ))?;
        
        // Create the OpenTelemetry layer
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
        layers.push(otel_layer.boxed());
        
        Some(tracer)
    } else {
        None
    };
    
    // Build the subscriber
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(layers);
    
    // Set the subscriber as the global default
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| ObservabilityError::TracingInitFailed(
            format!("Failed to set global subscriber: {}", e)
        ))?;
    
    Ok(TracingGuard {
        _file_guard: file_guard,
        _jaeger_pipeline: jaeger_pipeline,
    })
}