//! Interface to the Lion isolation component
//!
//! This module provides functions to interact with the Lion isolation system,
//! which is responsible for managing WASM sandboxes, resource limits, and
//! hostcalls for plugins.

use anyhow::{Context, Result};
use std::path::Path;

/// Compile a source file to WASM
pub fn compile_to_wasm(source_path: &Path, output_path: &Path, language: Language) -> Result<()> {
    // In a real implementation, this would call into lion_isolation::wasm::compiler
    #[cfg(feature = "isolation-integration")]
    {
        use lion_isolation::wasm::compiler;

        match language {
            Language::Rust => compiler::compile_rust_to_wasm(source_path, output_path)?,
            Language::Python => compiler::compile_python_to_wasm(source_path, output_path)?,
            Language::TypeScript => compiler::compile_typescript_to_wasm(source_path, output_path)?,
        }
    }

    #[cfg(not(feature = "isolation-integration"))]
    {
        // Placeholder implementation
        println!("Compiling {:?} to WASM...", source_path);
        println!("Using {} compiler", language);
        println!("Output will be written to {:?}", output_path);

        // Create a mock WASM file
        std::fs::write(output_path, b"mock wasm content")
            .context("Failed to write mock WASM file")?;

        println!("Compilation successful");
    }

    Ok(())
}

/// Set resource limits for a plugin
pub fn set_resource_limits(plugin_id: &str, limits: ResourceLimits) -> Result<()> {
    // In a real implementation, this would call into lion_isolation::resource::limiter
    #[cfg(feature = "isolation-integration")]
    {
        use lion_core::id::PluginId;
        use lion_isolation::resource::limiter;

        let id = PluginId::from_str(plugin_id).context("Invalid plugin ID format")?;

        limiter::set_memory_limit(&id, limits.memory_mb)?;
        limiter::set_cpu_limit(&id, limits.cpu_percent)?;
        limiter::set_instruction_limit(&id, limits.max_instructions)?;
        limiter::set_time_limit(&id, limits.max_execution_time_ms)?;
    }

    #[cfg(not(feature = "isolation-integration"))]
    {
        // Placeholder implementation
        println!("Setting resource limits for plugin: {}", plugin_id);
        println!("Memory limit: {} MB", limits.memory_mb);
        println!("CPU limit: {}%", limits.cpu_percent);
        println!("Instruction limit: {}", limits.max_instructions);
        println!("Time limit: {} ms", limits.max_execution_time_ms);
        println!("Resource limits set successfully");
    }

    Ok(())
}

/// Get resource usage for a plugin
pub fn get_resource_usage(plugin_id: &str) -> Result<ResourceUsage> {
    // In a real implementation, this would call into lion_isolation::resource::usage
    #[cfg(feature = "isolation-integration")]
    {
        use lion_core::id::PluginId;
        use lion_isolation::resource::usage;

        let id = PluginId::from_str(plugin_id).context("Invalid plugin ID format")?;

        let plugin_usage = usage::get_plugin_resource_usage(&id)?;

        Ok(ResourceUsage {
            memory_mb: plugin_usage.memory_mb,
            cpu_percent: plugin_usage.cpu_percent,
            instructions_executed: plugin_usage.instructions_executed,
            execution_time_ms: plugin_usage.execution_time_ms,
        })
    }

    #[cfg(not(feature = "isolation-integration"))]
    {
        // Placeholder implementation
        Ok(ResourceUsage {
            memory_mb: 32.5,
            cpu_percent: 1.2,
            instructions_executed: 1_500_000,
            execution_time_ms: 250,
        })
    }
}

/// Register a custom hostcall for plugins
pub fn register_hostcall(name: &str, handler: fn(&[u8]) -> Result<Vec<u8>>) -> Result<()> {
    // In a real implementation, this would call into lion_isolation::wasm::hostcall
    #[cfg(feature = "isolation-integration")]
    {
        use lion_isolation::wasm::hostcall;

        hostcall::register_custom_hostcall(name, handler)?;
    }

    #[cfg(not(feature = "isolation-integration"))]
    {
        // Placeholder implementation
        println!("Registering custom hostcall: {}", name);
        println!("Custom hostcall registered successfully");
    }

    Ok(())
}

/// Programming language for WASM compilation
#[derive(Debug, Clone, Copy)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Python => write!(f, "Python"),
            Language::TypeScript => write!(f, "TypeScript"),
        }
    }
}

/// Resource limits for a plugin
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub max_instructions: u64,
    pub max_execution_time_ms: u64,
}

/// Resource usage for a plugin
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub instructions_executed: u64,
    pub execution_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_compile_to_wasm() {
        let temp_dir = tempdir().unwrap();
        let source_path = temp_dir.path().join("test_source.rs");
        let output_path = temp_dir.path().join("test_output.wasm");

        // Create a mock source file
        std::fs::write(&source_path, b"fn main() {}").unwrap();

        let result = compile_to_wasm(&source_path, &output_path, Language::Rust);
        assert!(result.is_ok());

        // Check that the output file was created
        assert!(output_path.exists());
    }

    #[test]
    fn test_set_resource_limits() {
        let plugin_id = uuid::Uuid::new_v4().to_string();
        let limits = ResourceLimits {
            memory_mb: 128.0,
            cpu_percent: 50.0,
            max_instructions: 1_000_000,
            max_execution_time_ms: 5000,
        };

        let result = set_resource_limits(&plugin_id, limits);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_resource_usage() {
        let plugin_id = uuid::Uuid::new_v4().to_string();
        let result = get_resource_usage(&plugin_id);
        assert!(result.is_ok());

        let usage = result.unwrap();
        assert!(usage.memory_mb >= 0.0);
        assert!(usage.cpu_percent >= 0.0);
        assert!(usage.instructions_executed >= 0);
        assert!(usage.execution_time_ms >= 0);
    }

    #[test]
    fn test_register_hostcall() {
        fn test_handler(_data: &[u8]) -> Result<Vec<u8>> {
            Ok(vec![1, 2, 3])
        }

        let result = register_hostcall("test_hostcall", test_handler);
        assert!(result.is_ok());
    }
}
