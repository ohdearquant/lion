//! Formatting utilities for the CLI output.

use colored::Colorize;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Row, Table};
use lion_core::capability::CoreCapability;
use lion_core::plugin::{PluginId, PluginState};
use lion_core::resource::ResourceUsage;
use std::time::Duration;

/// Format a plugin ID for display
pub fn format_plugin_id(id: PluginId) -> String {
    id.0.to_string()
}

/// Format a plugin state for display with color
pub fn format_plugin_state(state: PluginState) -> String {
    match state {
        PluginState::Created => "Created".yellow().to_string(),
        PluginState::Initializing => "Initializing".yellow().to_string(),
        PluginState::Ready => "Ready".green().to_string(),
        PluginState::Processing => "Processing".blue().to_string(),
        PluginState::Paused => "Paused".yellow().to_string(),
        PluginState::Failed => "Failed".red().to_string(),
        PluginState::Terminated => "Terminated".red().to_string(),
    }
}

/// Format a capability for display
pub fn format_capability(capability: &CoreCapability) -> String {
    match capability {
        CoreCapability::FileSystemRead { path } => {
            if let Some(path) = path {
                format!("FileSystemRead ({})", path)
            } else {
                "FileSystemRead (all)".to_string()
            }
        }
        CoreCapability::FileSystemWrite { path } => {
            if let Some(path) = path {
                format!("FileSystemWrite ({})", path)
            } else {
                "FileSystemWrite (all)".to_string()
            }
        }
        CoreCapability::NetworkClient { hosts } => {
            if let Some(hosts) = hosts {
                format!("NetworkClient ({})", hosts.join(", "))
            } else {
                "NetworkClient (all)".to_string()
            }
        }
        CoreCapability::InterPluginComm => "InterPluginComm".to_string(),
    }
}

/// Format resource usage for display
pub fn format_resource_usage(usage: &ResourceUsage) -> String {
    format!(
        "Memory: {} bytes (peak: {} bytes)\nCPU: {:.2}%\nExecution time: {}\nMessages processed: {}",
        usage.memory_bytes,
        usage.peak_memory_bytes,
        usage.cpu_usage * 100.0,
        format_duration(usage.execution_time),
        usage.messages_processed
    )
}

/// Format a duration for display
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let millis = duration.subsec_millis();
    
    if hours > 0 {
        format!("{}h {}m {}s {}ms", hours, minutes, seconds, millis)
    } else if minutes > 0 {
        format!("{}m {}s {}ms", minutes, seconds, millis)
    } else if seconds > 0 {
        format!("{}s {}ms", seconds, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Create a table for plugin listing
pub fn create_plugin_table() -> Table {
    let mut table = Table::new();
    table
        .set_header(vec!["ID", "Name", "State", "Memory", "CPU"])
        .set_content_arrangement(ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    
    table
}

/// Add a plugin row to a table
pub fn add_plugin_row(
    table: &mut Table,
    id: PluginId,
    name: &str,
    state: PluginState,
    memory_bytes: usize,
    cpu_usage: f64,
) {
    table.add_row(vec![
        Cell::new(format_plugin_id(id)),
        Cell::new(name),
        Cell::new(format_plugin_state(state)),
        Cell::new(format_size(memory_bytes)),
        Cell::new(format!("{:.2}%", cpu_usage * 100.0)),
    ]);
}

/// Format a size in bytes for display
pub fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    const GB: usize = 1024 * MB;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Create a table for capabilities
pub fn create_capability_table() -> Table {
    let mut table = Table::new();
    table
        .set_header(vec!["Capability", "Details"])
        .set_content_arrangement(ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    
    table
}

/// Add a capability row to a table
pub fn add_capability_row(table: &mut Table, capability: &CoreCapability) {
    match capability {
        CoreCapability::FileSystemRead { path } => {
            table.add_row(vec![
                Cell::new("FileSystemRead".green()),
                Cell::new(path.clone().unwrap_or_else(|| "All paths".to_string())),
            ]);
        }
        CoreCapability::FileSystemWrite { path } => {
            table.add_row(vec![
                Cell::new("FileSystemWrite".yellow()),
                Cell::new(path.clone().unwrap_or_else(|| "All paths".to_string())),
            ]);
        }
        CoreCapability::NetworkClient { hosts } => {
            table.add_row(vec![
                Cell::new("NetworkClient".blue()),
                Cell::new(match hosts {
                    Some(hosts) => hosts.join(", "),
                    None => "All hosts".to_string(),
                }),
            ]);
        }
        CoreCapability::InterPluginComm => {
            table.add_row(vec![
                Cell::new("InterPluginComm".magenta()),
                Cell::new("Message passing between plugins"),
            ]);
        }
    }
}

/// Create a table for resources
pub fn create_resource_table() -> Table {
    let mut table = Table::new();
    table
        .set_header(vec!["Resource", "Value"])
        .set_content_arrangement(ContentArrangement::Dynamic)
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    
    table
}

/// Add resource rows to a table
pub fn add_resource_rows(table: &mut Table, usage: &ResourceUsage) {
    table.add_row(vec![
        Cell::new("Memory"),
        Cell::new(format_size(usage.memory_bytes)),
    ]);
    
    table.add_row(vec![
        Cell::new("Peak Memory"),
        Cell::new(format_size(usage.peak_memory_bytes)),
    ]);
    
    table.add_row(vec![
        Cell::new("CPU Usage"),
        Cell::new(format!("{:.2}%", usage.cpu_usage * 100.0)),
    ]);
    
    table.add_row(vec![
        Cell::new("Execution Time"),
        Cell::new(format_duration(usage.execution_time)),
    ]);
    
    table.add_row(vec![
        Cell::new("Messages Processed"),
        Cell::new(usage.messages_processed.to_string()),
    ]);
}

/// Create a progress bar
pub fn create_progress_bar(len: u64) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new(len);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
    );
    pb
}

/// Format a JSON value for pretty display
pub fn format_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}