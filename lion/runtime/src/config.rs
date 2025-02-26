//! Configuration for the Lion runtime.
//!
//! This module provides configuration management for the runtime,
//! including loading and saving configuration from files.

use std::path::{Path, PathBuf};
use std::net::SocketAddr;

use serde::{Serialize, Deserialize};
use core::error::{Result, Error};

/// Configuration for the Lion runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Maximum memory usage in bytes.
    pub max_memory_bytes: usize,
    
    /// Default minimum instances for concurrency.
    pub default_min_instances: usize,
    
    /// Default maximum instances for concurrency.
    pub default_max_instances: usize,
    
    /// Default wait timeout for instance acquisition in milliseconds.
    pub default_wait_timeout_ms: u64,
    
    /// Default idle timeout for instances in seconds.
    pub default_idle_timeout_sec: u64,
    
    /// Default maximum parallel nodes for workflows.
    pub default_max_parallel_nodes: usize,
    
    /// Default workflow timeout in milliseconds.
    pub default_workflow_timeout_ms: u64,
    
    /// Default continue on failure flag for workflows.
    pub default_continue_on_failure: bool,
    
    /// Default use checkpoints flag for workflows.
    pub default_use_checkpoints: bool,
    
    /// Default checkpoint interval in milliseconds.
    pub default_checkpoint_interval_ms: u64,
    
    /// Maximum active workflow executions.
    pub max_active_executions: usize,
    
    /// Whether to enable observability.
    #[serde(default)]
    pub enable_observability: bool,
    
    /// Log level for tracing.
    #[serde(default = "default_log_level")]
    pub log_level: String,
    
    /// Whether to enable file logging.
    #[serde(default)]
    pub enable_file_logging: bool,
    
    /// Directory for log files.
    #[serde(default)]
    pub log_directory: Option<PathBuf>,
    
    /// Whether to enable console logging.
    #[serde(default = "default_true")]
    pub enable_console_logging: bool,
    
    /// Whether to enable JSON formatting for logs.
    #[serde(default)]
    pub enable_json_format: bool,
    
    /// Whether to enable Jaeger tracing.
    #[serde(default)]
    pub enable_jaeger: bool,
    
    /// Jaeger endpoint.
    #[serde(default)]
    pub jaeger_endpoint: Option<String>,
    
    /// Whether to enable Prometheus metrics.
    #[serde(default)]
    pub enable_prometheus: bool,
    
    /// Prometheus listen address.
    #[serde(default)]
    pub prometheus_addr: Option<SocketAddr>,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 100 * 1024 * 1024, // 100 MB
            default_min_instances: 1,
            default_max_instances: 10,
            default_wait_timeout_ms: 100,
            default_idle_timeout_sec: 60,
            default_max_parallel_nodes: 4,
            default_workflow_timeout_ms: 300000, // 5 minutes
            default_continue_on_failure: false,
            default_use_checkpoints: false,
            default_checkpoint_interval_ms: 60000, // 1 minute
            max_active_executions: 100,
            enable_observability: false,
            log_level: "info".to_string(),
            enable_file_logging: false,
            log_directory: None,
            enable_console_logging: true,
            enable_json_format: false,
            enable_jaeger: false,
            jaeger_endpoint: None,
            enable_prometheus: false,
            prometheus_addr: None,
        }
    }
}

/// Load configuration from a file.
pub fn load_config(path: &Path) -> Result<RuntimeConfig> {
    // Check file extension
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("toml");
    
    // Read the file
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Runtime(format!("Failed to read config file: {}", e)))?;
    
    // Parse the file
    match ext {
        "toml" => {
            toml::from_str(&content)
                .map_err(|e| Error::Runtime(format!("Failed to parse TOML config: {}", e)))
        },
        "yaml" | "yml" => {
            serde_yaml::from_str(&content)
                .map_err(|e| Error::Runtime(format!("Failed to parse YAML config: {}", e)))
        },
        "json" => {
            serde_json::from_str(&content)
                .map_err(|e| Error::Runtime(format!("Failed to parse JSON config: {}", e)))
        },
        _ => Err(Error::Runtime(format!("Unsupported config file format: {}", ext))),
    }
}

/// Save configuration to a file.
pub fn save_config(config: &RuntimeConfig, path: &Path) -> Result<()> {
    // Check file extension
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("toml");
    
    // Serialize the config
    let content = match ext {
        "toml" => {
            toml::to_string_pretty(config)
                .map_err(|e| Error::Runtime(format!("Failed to serialize TOML config: {}", e)))?
        },
        "yaml" | "yml" => {
            serde_yaml::to_string(config)
                .map_err(|e| Error::Runtime(format!("Failed to serialize YAML config: {}", e)))?
        },
        "json" => {
            serde_json::to_string_pretty(config)
                .map_err(|e| Error::Runtime(format!("Failed to serialize JSON config: {}", e)))?
        },
        _ => return Err(Error::Runtime(format!("Unsupported config file format: {}", ext))),
    };
    
    // Write the file
    std::fs::write(path, content)
        .map_err(|e| Error::Runtime(format!("Failed to write config file: {}", e)))?;
    
    Ok(())
}