//! Workflow management commands
//!
//! This module contains commands for workflow management.
//! It is currently under development and not all features are implemented.

use super::interfaces::workflow;
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;

/// Register a new workflow from a definition file
pub fn register_workflow(file_path: &Path) -> Result<String> {
    // Use the workflow interface to register the workflow
    println!(
        "Registering workflow from file: {}",
        file_path.display().to_string().cyan()
    );

    let workflow_id = workflow::register_workflow(file_path).context(format!(
        "Failed to register workflow from {}",
        file_path.display()
    ))?;

    println!("{}", "Workflow registered successfully".green().bold());
    println!("Workflow ID: {}", workflow_id.cyan());

    // Show relevant next commands
    println!("\n{}", "Next steps:".bold());
    println!(
        "  {}",
        format!("lion-cli workflow start {}", workflow_id).italic()
    );
    println!(
        "  {}",
        format!("lion-cli workflow status {}", workflow_id).italic()
    );

    Ok(workflow_id)
}

/// Start a registered workflow
pub fn start_workflow(workflow_id: &str) -> Result<()> {
    // Use the workflow interface to start the workflow
    println!("Starting workflow: {}", workflow_id.cyan());

    workflow::start_workflow(workflow_id)
        .context(format!("Failed to start workflow {}", workflow_id))?;

    println!("{}", "Workflow started successfully".green());
    println!("\n{}", "To check status:".bold());
    println!(
        "  {}",
        format!("lion-cli workflow status {}", workflow_id).italic()
    );

    Ok(())
}

/// Pause a running workflow
pub fn pause_workflow(workflow_id: &str) -> Result<()> {
    // Use the workflow interface to pause the workflow
    println!("Pausing workflow: {}", workflow_id.cyan());

    workflow::pause_workflow(workflow_id)
        .context(format!("Failed to pause workflow {}", workflow_id))?;

    println!("{}", "Workflow paused successfully".yellow());
    println!("\n{}", "To resume:".bold());
    println!(
        "  {}",
        format!("lion-cli workflow resume {}", workflow_id).italic()
    );

    Ok(())
}

/// Resume a paused workflow
pub fn resume_workflow(workflow_id: &str) -> Result<()> {
    // Use the workflow interface to resume the workflow
    println!("Resuming workflow: {}", workflow_id.cyan());

    workflow::resume_workflow(workflow_id)
        .context(format!("Failed to resume workflow {}", workflow_id))?;

    println!("{}", "Workflow resumed successfully".green());

    Ok(())
}

/// Check workflow status
pub fn check_workflow_status(workflow_id: &str) -> Result<()> {
    // Use the workflow interface to get the workflow status
    let status = workflow::get_workflow_status(workflow_id)
        .context(format!("Failed to get status for workflow {}", workflow_id))?;

    println!("{}", "Workflow Status".underline().bold());
    println!("ID:    {}", workflow_id.cyan());

    // Color the state based on its value
    let state_colored = match status.state.to_lowercase().as_str() {
        "running" => status.state.green().bold(),
        "paused" => status.state.yellow().bold(),
        "completed" => status.state.cyan().bold(),
        "failed" => status.state.red().bold(),
        _ => status.state.normal(),
    };

    println!("State: {}", state_colored);

    // Format progress as a percentage and progress bar
    let progress_percent =
        (status.current_step as f64 / status.total_steps as f64 * 100.0).round() as i32;
    let progress_bar = format!(
        "[{}{}] {}%",
        "=".repeat((progress_percent / 5) as usize),
        " ".repeat((20 - (progress_percent / 5)) as usize),
        progress_percent
    );

    println!(
        "Progress: {} ({}/{})",
        progress_bar.yellow(),
        status.current_step,
        status.total_steps
    );
    println!("Current node: '{}'", status.current_node.bright_white());
    println!("Started at: {}", status.started_at.dimmed());

    // Calculate running time in a human-readable format
    let seconds = status.running_time_seconds;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let runtime_str = if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    };

    println!("Running for: {}", runtime_str.bright_green());

    if let Some(error) = status.error {
        println!("Error: {}", error.red());
    }

    // Display available actions based on current state
    println!("\n{}", "Available actions:".bold());
    match status.state.to_lowercase().as_str() {
        "running" => println!(
            "  {}",
            format!("lion-cli workflow pause {}", workflow_id).italic()
        ),
        "paused" => println!(
            "  {}",
            format!("lion-cli workflow resume {}", workflow_id).italic()
        ),
        _ => {}
    }
    println!(
        "  {}",
        format!("lion-cli workflow cancel {}", workflow_id).italic()
    );

    Ok(())
}

/// Cancel a running workflow
pub fn cancel_workflow(workflow_id: &str) -> Result<()> {
    println!("Cancelling workflow: {}", workflow_id.cyan());

    // Use the workflow interface to cancel the workflow
    workflow::cancel_workflow(workflow_id)
        .context(format!("Failed to cancel workflow {}", workflow_id))?;

    println!("{}", "Workflow cancelled successfully".red());
    Ok(())
}

/// List all registered workflows
pub fn list_workflows() -> Result<()> {
    // Use the workflow interface to list all workflows
    let workflow_list = workflow::list_workflows().context("Failed to retrieve workflow list")?;

    if workflow_list.is_empty() {
        println!("{}", "No workflows registered yet".yellow());
        println!("\nTo register a workflow:");
        println!(
            "  {}",
            "lion-cli workflow register --file <path/to/workflow.json>".italic()
        );
        return Ok(());
    }

    // Store the count for later use
    let workflow_count = workflow_list.len();

    // Print the header
    println!("{}", "Registered Workflows".green().bold());
    println!(
        "{:<36} | {:<14} | {:<42} | {:^5} | {:^5}",
        "ID".underline(),
        "Name".underline(),
        "Description".underline(),
        "Nodes".underline(),
        "Edges".underline()
    );

    println!("{}", "-".repeat(110));

    // Print each workflow entry
    for wf in &workflow_list {
        // Truncate long descriptions
        let description = if wf.description.len() > 42 {
            format!("{}...", &wf.description[..39])
        } else {
            wf.description.clone()
        };

        // Print the formatted workflow information
        println!(
            "{} | {:<14} | {:<42} | {:^5} | {:^5}",
            wf.id.cyan(),
            wf.name,
            description,
            wf.node_count,
            wf.edge_count
        );
    }

    println!(
        "\n{} registered workflow(s) found",
        workflow_count.to_string().yellow()
    );

    Ok(())
}
