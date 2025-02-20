use agentic_core::{Orchestrator, PluginManifest, SystemEvent};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::fs;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Load a plugin from a manifest file
    LoadPlugin {
        /// Path to the plugin manifest file
        manifest_path: String,
        /// Wait for plugin load completion
        #[arg(long)]
        wait: bool,
    },
    /// List loaded plugins
    ListPlugins,
    /// Invoke a plugin function
    InvokePlugin {
        /// Plugin ID
        plugin_id: String,
        /// Function name (e.g., add, subtract)
        function: String,
        /// First number
        #[arg(long)]
        a: f64,
        /// Second number
        #[arg(long)]
        b: f64,
    },
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Create an orchestrator with a channel size of 100
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn the orchestrator in a separate task
    tokio::spawn(orchestrator.run());

    match cli.command {
        Commands::LoadPlugin {
            manifest_path,
            wait,
        } => {
            info!("Loading plugin from manifest: {}", manifest_path);

            // Read and parse the manifest file
            let manifest_content = fs::read_to_string(&manifest_path).unwrap_or_else(|e| {
                panic!("Failed to read manifest file: {}", e);
            });
            debug!("Manifest content: {}", manifest_content);

            let manifest: PluginManifest = toml::from_str(&manifest_content).unwrap_or_else(|e| {
                panic!("Failed to parse manifest: {}", e);
            });
            debug!("Parsed manifest: {:#?}", manifest);

            // Generate a plugin ID
            let plugin_id = Uuid::new_v4();
            debug!("Sending PluginLoad event with ID: {}", plugin_id);

            // Send the load event
            sender
                .send(SystemEvent::PluginLoad {
                    plugin_id,
                    manifest,
                    manifest_path: Some(manifest_path),
                })
                .await
                .unwrap();

            println!("Plugin load initiated with ID: {}", plugin_id);

            if wait {
                debug!("Waiting for plugin load completion...");
                // Wait for completion event
                while let Ok(event) = completion_rx.recv().await {
                    debug!("Received event: {:#?}", event);
                    match event {
                        SystemEvent::PluginResult {
                            plugin_id: id,
                            result,
                        } if id == plugin_id => {
                            println!("Plugin loaded successfully: {}", result);
                            break;
                        }
                        SystemEvent::PluginError {
                            plugin_id: id,
                            error,
                        } if id == plugin_id => {
                            println!("Plugin load failed: {}", error);
                            break;
                        }
                        _ => continue,
                    }
                }
            }
        }
        Commands::ListPlugins => {
            info!("Listing plugins");
            sender.send(SystemEvent::ListPlugins).await.unwrap();

            // Wait for response
            while let Ok(event) = completion_rx.recv().await {
                debug!("Received event: {:#?}", event);
                match event {
                    SystemEvent::PluginResult { result, .. } => {
                        println!("{}", result);
                        break;
                    }
                    _ => continue,
                }
            }
        }
        Commands::InvokePlugin {
            plugin_id,
            function,
            a,
            b,
        } => {
            let plugin_id = Uuid::parse_str(&plugin_id).unwrap_or_else(|e| {
                panic!("Invalid plugin ID: {}", e);
            });

            // Create input JSON with function name and args
            let input = json!({
                "function": function,
                "args": {
                    "a": a,
                    "b": b
                }
            })
            .to_string();

            debug!("Invoking plugin with input: {}", input);

            sender
                .send(SystemEvent::PluginInvoked { plugin_id, input })
                .await
                .unwrap();

            // Wait for response
            while let Ok(event) = completion_rx.recv().await {
                debug!("Received event: {:#?}", event);
                match event {
                    SystemEvent::PluginResult {
                        plugin_id: id,
                        result,
                    } if id == plugin_id => {
                        println!("Plugin result: {}", result);
                        break;
                    }
                    SystemEvent::PluginError {
                        plugin_id: id,
                        error,
                    } if id == plugin_id => {
                        println!("Plugin error: {}", error);
                        break;
                    }
                    _ => continue,
                }
            }
        }
    }
}
