use agentic_core::{ElementData, InMemoryStore};
use clap::{Parser, Subcommand};
use serde_json::Value;
use tracing::{error, info};

#[derive(Debug, Parser)]
#[command(name = "lion-cli", version = "0.0.1-alpha")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new element with the given metadata
    CreateElement {
        /// JSON metadata for the element
        #[arg(long)]
        metadata: String,
    },
    /// List all stored element IDs
    ListElements,
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    // Create an ephemeral store (Phase 1 is purely in-memory)
    let store = InMemoryStore::new();

    match cli.command {
        Commands::CreateElement { metadata } => match serde_json::from_str::<Value>(&metadata) {
            Ok(parsed) => {
                let elem = ElementData::new(parsed);
                let id = store.create_element(elem);
                info!("Created element with ID: {}", id);
                println!("Created element with ID: {}", id);
            }
            Err(e) => {
                error!("Invalid JSON metadata: {}", e);
                eprintln!("Error: Invalid JSON for --metadata");
                std::process::exit(1);
            }
        },
        Commands::ListElements => {
            let ids = store.list_element_ids();
            if ids.is_empty() {
                println!("No elements stored yet.");
            } else {
                println!("Element IDs:");
                for id in ids {
                    println!("  {}", id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_store_operations() {
        let store = InMemoryStore::new();
        assert!(store.is_empty());

        // Create an element
        let metadata = json!({"test": "value"});
        let elem = ElementData::new(metadata);
        let id = store.create_element(elem);

        // Verify it exists
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);

        let ids = store.list_element_ids();
        assert!(ids.contains(&id));
    }
}
