use crate::event_printer::print_event_log;
use lion_core::{
    orchestrator::{Orchestrator, SystemEvent},
    plugin_manager::{PluginManager, PluginManifest},
};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;

pub async fn handle_spawn_agent(prompt: String, correlation_id: Option<String>) {
    // Create orchestrator with a channel capacity of 100
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Get a clone of the event log before moving orchestrator
    let event_log = Arc::new(orchestrator.event_log().clone());

    // Spawn the orchestrator's event loop
    tokio::spawn(orchestrator.run());

    let correlation_uuid = correlation_id
        .map(|id| Uuid::parse_str(&id))
        .transpose()
        .expect("Invalid correlation ID format");

    println!("\n=== Spawning Agent ===");
    let event = SystemEvent::new_agent(prompt, correlation_uuid);

    // Extract agent_id before moving event
    let agent_id = match &event {
        SystemEvent::AgentSpawned { agent_id, .. } => *agent_id,
        _ => panic!("Unexpected event type"),
    };

    // Send the event
    if let Err(e) = sender.send(event).await {
        eprintln!("Failed to spawn agent: {}", e);
        std::process::exit(1);
    }

    println!("Agent spawned successfully with ID: {}", agent_id);

    // Wait for completion with timeout
    match timeout(Duration::from_secs(5), completion_rx.recv()).await {
        Ok(Ok(completion)) => match completion {
            SystemEvent::AgentCompleted { result, .. } => {
                println!("Agent completed successfully!");
                println!("Result: {}", result);
            }
            SystemEvent::AgentError { error, .. } => {
                eprintln!("Agent failed: {}", error);
                std::process::exit(1);
            }
            _ => {}
        },
        Ok(Err(e)) => {
            eprintln!("Error receiving completion: {}", e);
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("Timeout waiting for agent completion");
            std::process::exit(1);
        }
    }

    print_event_log(&event_log).await;
}

pub async fn handle_demo(data: String, correlation_id: Option<String>) {
    // Create orchestrator with a channel capacity of 100
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Get a clone of the event log before moving orchestrator
    let event_log = Arc::new(orchestrator.event_log().clone());

    // Spawn the orchestrator's event loop
    tokio::spawn(orchestrator.run());

    let correlation_uuid = correlation_id
        .map(|id| Uuid::parse_str(&id))
        .transpose()
        .expect("Invalid correlation ID format");

    println!("\n=== Submitting Task ===");
    let event = SystemEvent::new_task(data, correlation_uuid);

    // Extract task_id before moving event
    let task_id = match &event {
        SystemEvent::TaskSubmitted { task_id, .. } => *task_id,
        _ => panic!("Unexpected event type"),
    };

    // Send the event
    if let Err(e) = sender.send(event).await {
        eprintln!("Failed to submit task: {}", e);
        std::process::exit(1);
    }

    println!("Task submitted successfully with ID: {}", task_id);

    // Wait for completion with timeout
    match timeout(Duration::from_secs(5), completion_rx.recv()).await {
        Ok(Ok(completion)) => match completion {
            SystemEvent::TaskCompleted { result, .. } => {
                println!("Task completed successfully!");
                println!("Result: {}", result);
            }
            SystemEvent::TaskError { error, .. } => {
                eprintln!("Task failed: {}", error);
                std::process::exit(1);
            }
            _ => {}
        },
        Ok(Err(e)) => {
            eprintln!("Error receiving completion: {}", e);
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("Timeout waiting for task completion");
            std::process::exit(1);
        }
    }

    print_event_log(&event_log).await;
}

pub fn handle_load_plugin(manifest: String) {
    let mut plugin_manager = PluginManager::with_manifest_dir("plugins");

    // Read and parse the manifest file
    let manifest_content = std::fs::read_to_string(manifest).expect("Failed to read manifest file");
    let manifest: PluginManifest =
        toml::from_str(&manifest_content).expect("Failed to parse manifest");

    println!("\n=== Loading Plugin ===");
    println!("Name: {}", manifest.name);
    println!("Version: {}", manifest.version);
    println!("Entry Point: {}", manifest.entry_point);

    // Load the plugin
    match plugin_manager.load_plugin(manifest) {
        Ok(plugin_id) => {
            println!("\nPlugin loaded successfully!");
            println!("Plugin ID: {}", plugin_id);
        }
        Err(e) => {
            eprintln!("Failed to load plugin: {}", e);
            std::process::exit(1);
        }
    }
}

pub async fn handle_invoke_plugin(
    plugin_id: String,
    input: String,
    correlation_id: Option<String>,
) {
    let plugin_uuid = Uuid::parse_str(&plugin_id).expect("Invalid plugin ID format");
    let correlation_uuid = correlation_id
        .map(|id| Uuid::parse_str(&id))
        .transpose()
        .expect("Invalid correlation ID format");

    // Initialize plugin manager with plugins directory
    let mut plugin_manager = PluginManager::with_manifest_dir("plugins");

    // Discover and load available plugins
    match plugin_manager.discover_plugins() {
        Ok(manifests) => {
            for manifest in manifests {
                if let Err(e) = plugin_manager.load_plugin(manifest) {
                    eprintln!("Warning: Failed to load plugin: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to discover plugins: {}", e);
        }
    }

    let orchestrator = Orchestrator::with_plugin_manager(100, plugin_manager);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Get a clone of the event log before moving orchestrator
    let event_log = Arc::new(orchestrator.event_log().clone());

    // Spawn the orchestrator
    tokio::spawn(orchestrator.run());

    println!("\n=== Invoking Plugin ===");
    let event = SystemEvent::new_plugin_invocation(plugin_uuid, input, correlation_uuid);

    // Send the event
    if let Err(e) = sender.send(event).await {
        eprintln!("Failed to invoke plugin: {}", e);
        std::process::exit(1);
    }

    println!("Plugin invocation sent successfully");

    // Wait for completion with timeout
    match timeout(Duration::from_secs(5), completion_rx.recv()).await {
        Ok(Ok(completion)) => match completion {
            SystemEvent::PluginResult { output, .. } => {
                println!("Plugin completed successfully!");
                println!("Output: {}", output);
            }
            SystemEvent::PluginError { error, .. } => {
                eprintln!("Plugin failed: {}", error);
                std::process::exit(1);
            }
            _ => {}
        },
        Ok(Err(e)) => {
            eprintln!("Error receiving completion: {}", e);
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("Timeout waiting for plugin completion");
            std::process::exit(1);
        }
    }

    print_event_log(&event_log).await;
}

pub fn handle_ci() {
    let status = std::process::Command::new("./scripts/ci.sh")
        .status()
        .expect("Failed to execute CI script");
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}

pub fn handle_test_cli() {
    let status = std::process::Command::new("./scripts/test_cli.sh")
        .status()
        .expect("Failed to execute test script");
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
