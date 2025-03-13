use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Make sure we have access to the lib crate
extern crate lion_core;
mod commands;
use commands::{plugin, policy, system, workflow};

/// Lion Command Line Interface
///
/// This CLI is currently in development and not all features are implemented yet.
#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Plugin management commands
    Plugin {
        #[clap(subcommand)]
        cmd: PluginCommands,
    },

    /// System management commands
    Policy {
        #[clap(subcommand)]
        cmd: PolicyCommands,
    },

    /// System management commands
    System {
        #[clap(subcommand)]
        cmd: SystemCommands,
    },

    /// Workflow management commands
    Workflow {
        #[clap(subcommand)]
        cmd: WorkflowCommands,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Load a WASM plugin
    Load {
        /// Path to the WASM plugin file
        #[clap(long)]
        path: PathBuf,

        /// Path to capability configuration file (optional)
        #[clap(long)]
        caps: Option<PathBuf>,
    },

    /// List all loaded plugins
    List,

    /// Call a function in a loaded plugin
    Call {
        /// Plugin ID
        plugin_id: String,

        /// Function name to call
        function: String,

        /// Arguments as JSON string
        #[clap(long)]
        args: Option<String>,
    },

    /// Unload a plugin
    Unload {
        /// Plugin ID to unload
        plugin_id: String,
    },

    /// Grant capabilities to a plugin
    GrantCap {
        /// Plugin ID
        #[clap(long)]
        plugin: String,

        /// Capability type (e.g., file, network)
        #[clap(long)]
        cap_type: String,

        /// Capability parameters as JSON
        #[clap(long)]
        params: String,
    },
}

#[derive(Subcommand)]
enum PolicyCommands {
    /// Add a new policy rule
    Add {
        /// Unique ID for the rule
        #[clap(long)]
        rule_id: String,

        /// Subject of the rule (e.g., "plugin:123e4567-e89b-12d3-a456-426614174000")
        #[clap(long)]
        subject: String,

        /// Object of the rule (e.g., "file:/etc", "network:*")
        #[clap(long)]
        object: String,

        /// Action to take (allow, deny, audit)
        #[clap(long)]
        action: String,
    },

    /// List all policy rules
    List,

    /// Remove a policy rule
    Remove {
        /// ID of the rule to remove
        rule_id: String,
    },

    /// Check if a policy would allow a specific action
    Check {
        /// Subject (e.g., "plugin:123e4567-e89b-12d3-a456-426614174000")
        #[clap(long)]
        subject: String,

        /// Object (e.g., "file:/etc/passwd")
        #[clap(long)]
        object: String,

        /// Action to check (e.g., "read", "write", "connect")
        #[clap(long)]
        action: String,
    },
}

#[derive(Subcommand)]
enum SystemCommands {
    /// Start the Lion microkernel
    Start,

    /// Show system status
    Status,

    /// View system logs
    Logs {
        /// Filter logs by level (debug, info, warn, error)
        #[clap(long)]
        level: Option<String>,

        /// Filter logs by component
        #[clap(long)]
        component: Option<String>,
    },

    /// Shutdown the microkernel
    Shutdown,
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Register a new workflow
    Register {
        /// Path to workflow definition file
        #[clap(long)]
        file: PathBuf,
    },

    /// Start a registered workflow
    Start {
        /// Workflow ID
        workflow_id: String,
    },

    /// Pause a running workflow
    Pause {
        /// Workflow ID
        workflow_id: String,
    },

    /// Resume a paused workflow
    Resume {
        /// Workflow ID
        workflow_id: String,
    },

    /// Check workflow status
    Status {
        /// Workflow ID
        workflow_id: String,
    },

    /// Cancel a running workflow
    Cancel {
        /// Workflow ID
        workflow_id: String,
    },

    /// List all registered workflows
    List {},
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Plugin { cmd } => handle_plugin_command(cmd),
        Commands::Policy { cmd } => handle_policy_command(cmd),
        Commands::System { cmd } => handle_system_command(cmd),
        Commands::Workflow { cmd } => handle_workflow_command(cmd),
    }
}

fn handle_plugin_command(cmd: PluginCommands) -> Result<()> {
    match cmd {
        PluginCommands::Load { path, caps } => {
            let caps_path = caps.as_deref();
            let plugin_id = plugin::load_plugin(&path, caps_path)?;
            println!("Plugin loaded successfully with ID: {}", plugin_id);
            Ok(())
        }
        PluginCommands::List => plugin::list_plugins(),
        PluginCommands::Call {
            plugin_id,
            function,
            args,
        } => plugin::call_plugin(&plugin_id, &function, args.as_deref()),
        PluginCommands::Unload { plugin_id } => plugin::unload_plugin(&plugin_id),
        PluginCommands::GrantCap {
            plugin,
            cap_type,
            params,
        } => plugin::grant_capability(&plugin, &cap_type, &params),
    }
}

fn handle_policy_command(cmd: PolicyCommands) -> Result<()> {
    match cmd {
        PolicyCommands::Add {
            rule_id,
            subject,
            object,
            action,
        } => policy::add_policy(&rule_id, &subject, &object, &action),
        PolicyCommands::List => policy::list_policies(),
        PolicyCommands::Remove { rule_id } => policy::remove_policy(&rule_id),
        PolicyCommands::Check {
            subject,
            object,
            action,
        } => policy::check_policy(&subject, &object, &action),
    }
}

fn handle_system_command(cmd: SystemCommands) -> Result<()> {
    match cmd {
        SystemCommands::Start => system::start_system(),
        SystemCommands::Status => system::show_status(),
        SystemCommands::Logs { level, component } => {
            system::view_logs(level.as_deref(), component.as_deref())
        }
        SystemCommands::Shutdown => system::shutdown_system(),
    }
}

fn handle_workflow_command(cmd: WorkflowCommands) -> Result<()> {
    match cmd {
        WorkflowCommands::Register { file } => {
            let workflow_id = workflow::register_workflow(&file)?;
            println!("Workflow registered with ID: {}", workflow_id);
            Ok(())
        }
        WorkflowCommands::Start { workflow_id } => workflow::start_workflow(&workflow_id),
        WorkflowCommands::Pause { workflow_id } => workflow::pause_workflow(&workflow_id),
        WorkflowCommands::Resume { workflow_id } => workflow::resume_workflow(&workflow_id),
        WorkflowCommands::Status { workflow_id } => workflow::check_workflow_status(&workflow_id),
        WorkflowCommands::Cancel { workflow_id } => workflow::cancel_workflow(&workflow_id),
        WorkflowCommands::List {} => workflow::list_workflows(),
    }
}
