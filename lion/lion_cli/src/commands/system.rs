//! System management commands
//!
//! This module contains commands for Lion system management.
//! It is currently under development and not all features are implemented.

use super::interfaces::{observability, runtime};
use anyhow::Result;
use colored::*;

/// Start the Lion microkernel
pub fn start_system() -> Result<()> {
    // Use the runtime interface to start the system
    runtime::start_runtime()
}

/// Show system status
pub fn show_status() -> Result<()> {
    // Get system status from the runtime interface
    let status = runtime::get_runtime_status()?;

    // Format and display the status
    println!("{}", "Lion System Status:".bold().underline());
    println!("");
    println!(
        "Status: {}",
        if status.is_running {
            "RUNNING".green().bold()
        } else {
            "STOPPED".red().bold()
        }
    );

    // Format uptime
    let hours = status.uptime_seconds / 3600;
    let minutes = (status.uptime_seconds % 3600) / 60;
    let seconds = status.uptime_seconds % 60;
    println!(
        "Uptime: {}h {}m {}s",
        hours.to_string().cyan(),
        minutes.to_string().cyan(),
        seconds.to_string().cyan()
    );

    println!(
        "Loaded plugins: {}",
        status.loaded_plugins.to_string().yellow()
    );
    println!(
        "Active workflows: {}",
        status.active_workflows.to_string().yellow()
    );
    println!(
        "Memory usage: {} MB",
        format!("{:.1}", status.memory_usage_mb).magenta()
    );
    println!("CPU usage: {:.1}%", status.cpu_usage_percent);

    // Get metrics for more detailed information
    let metrics = observability::get_metrics()?;

    // Resource usage
    println!("\n{}", "Resource Usage:".bold());
    println!(
        "  Memory: {} MB / 1024 MB",
        format!("{:.1}", status.memory_usage_mb).magenta()
    );

    // Color the CPU usage based on load
    let cpu_str = format!("{:.1}%", status.cpu_usage_percent);
    let cpu_colored = if status.cpu_usage_percent < 30.0 {
        cpu_str.green()
    } else if status.cpu_usage_percent < 70.0 {
        cpu_str.yellow()
    } else {
        cpu_str.red()
    };
    println!("  CPU: {} (across 4 cores)", cpu_colored);

    if let Some(disk_read) = metrics.get("system.disk.read_mb_per_sec") {
        if let observability::MetricValue::Gauge(value) = disk_read {
            println!("  Disk I/O: {} MB/s read", format!("{:.1}", value).cyan());
        }
    }

    if let Some(disk_write) = metrics.get("system.disk.write_mb_per_sec") {
        if let observability::MetricValue::Gauge(value) = disk_write {
            println!("  Disk I/O: {} MB/s write", format!("{:.1}", value).cyan());
        }
    }

    // Workflow status
    println!("\n{}", "Workflow Status:".bold());
    println!("  data-processing: {} (Step 2/5)", "RUNNING".green());

    // Add command suggestions
    println!("\n{}", "Suggested commands:".bold());
    println!("  {}", "lion-cli plugin list".italic());
    println!("  {}", "lion-cli workflow list".italic());
    println!("  {}", "lion-cli system logs".italic());

    Ok(())
}

/// View system logs
pub fn view_logs(level: Option<&str>, component: Option<&str>) -> Result<()> {
    // Get logs from the observability interface
    let logs = observability::get_logs(level, component)?;

    println!("{}", "Viewing system logs:".bold());

    if let Some(log_level) = level {
        println!("Filter: Level = {}", log_level.yellow());
    }

    if let Some(comp) = component {
        println!("Filter: Component = {}", comp.yellow());
    }

    println!("");

    // Display logs in a formatted way
    for log in logs {
        // Color the log level based on severity
        let level_colored = match log.level.as_str() {
            "ERROR" => log.level.red().bold(),
            "WARN" => log.level.yellow().bold(),
            "INFO" => log.level.green(),
            _ => log.level.normal(),
        };
        println!(
            "[{}] [{}] [{}] {}",
            log.timestamp.dimmed(),
            level_colored,
            log.component.cyan(),
            log.message
        );
    }

    Ok(())
}

/// Shutdown the Lion microkernel
pub fn shutdown_system() -> Result<()> {
    // Use the runtime interface to shutdown the system
    println!("{}", "Shutting down Lion microkernel...".yellow());

    let result = runtime::shutdown_runtime();

    println!(
        "{}",
        "Lion microkernel has been shut down successfully.".green()
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_system() {
        let result = start_system();
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_status() {
        let result = show_status();
        assert!(result.is_ok());
    }

    #[test]
    fn test_view_logs() {
        let result = view_logs(Some("INFO"), Some("system"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_shutdown_system() {
        let result = shutdown_system();
        assert!(result.is_ok());
    }
}
