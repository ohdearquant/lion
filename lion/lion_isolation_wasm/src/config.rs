//! Configuration for the WebAssembly isolation backend.

use std::time::Duration;

/// Configuration for a WebAssembly instance
#[derive(Debug, Clone)]
pub struct WasmInstanceConfig {
    /// Maximum memory size in bytes
    pub memory_limit: usize,
    
    /// Maximum execution time for a single call
    pub execution_time_limit: Duration,
    
    /// Optional fuel limit (for instruction counting)
    pub fuel_limit: Option<u64>,
}

impl Default for WasmInstanceConfig {
    fn default() -> Self {
        Self {
            memory_limit: 100 * 1024 * 1024, // 100 MB
            execution_time_limit: Duration::from_secs(5),
            fuel_limit: Some(10_000_000), // 10 million instructions
        }
    }
}

/// Configuration for the WebAssembly isolation backend
#[derive(Debug, Clone)]
pub struct WasmIsolationConfig {
    /// Configuration for WebAssembly instances
    pub instance_config: WasmInstanceConfig,
    
    /// Whether to enable fuel-based metering
    pub enable_fuel_metering: bool,
    
    /// Default fuel limit
    pub default_fuel_limit: Option<u64>,
    
    /// Whether to cache compiled modules
    pub enable_module_caching: bool,
    
    /// Maximum number of cached modules
    pub max_cached_modules: usize,
    
    /// Whether to validate modules before compilation
    pub validate_modules: bool,
}

impl Default for WasmIsolationConfig {
    fn default() -> Self {
        Self {
            instance_config: WasmInstanceConfig::default(),
            enable_fuel_metering: true,
            default_fuel_limit: Some(10_000_000), // 10 million instructions
            enable_module_caching: true,
            max_cached_modules: 100,
            validate_modules: true,
        }
    }
}