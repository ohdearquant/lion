use crate::logs::LogEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use wasmtime::{Instance, Memory, Module};

/// WASM Plugin instance information
pub struct WasmPluginInstance {
    /// Plugin ID
    pub id: Uuid,

    /// Path to WASM file
    pub path: String,

    /// Compiled WASM module
    pub module: Option<Module>,

    /// WASM instance
    pub instance: Option<Instance>,

    /// WASM memory
    pub memory: Option<Memory>,

    /// Exported functions
    pub exports: Vec<String>,
}

/// Shared application state
pub struct AppState {
    /// Broadcast channel for real-time log events
    pub logs_tx: broadcast::Sender<LogEntry>,

    /// In-memory buffer of recent logs for search functionality
    pub log_buffer: Arc<RwLock<Vec<LogEntry>>>,

    /// Active agents
    pub agents: RwLock<HashMap<Uuid, String>>,

    /// Active plugins
    pub plugins: RwLock<HashMap<Uuid, PluginInfo>>,

    /// Active WASM plugin instances
    pub plugins_wasm: RwLock<HashMap<Uuid, WasmPluginInstance>>,

    /// WebAssembly engine
    pub wasm_engine: Option<wasmtime::Engine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
}

impl AppState {
    pub fn new(
        logs_tx: broadcast::Sender<LogEntry>,
        log_buffer: Arc<RwLock<Vec<LogEntry>>>,
    ) -> Self {
        // Initialize WebAssembly engine with default config
        let mut config = wasmtime::Config::new();
        config.wasm_reference_types(true);
        config.wasm_multi_value(true);
        config.async_support(true);

        let wasm_engine = wasmtime::Engine::new(&config).ok();

        Self {
            logs_tx,
            log_buffer,
            agents: RwLock::new(HashMap::new()),
            plugins: RwLock::new(HashMap::new()),
            plugins_wasm: RwLock::new(HashMap::new()),
            wasm_engine,
        }
    }

    /// Log a message to both the broadcast channel and the searchable buffer
    pub async fn log(&self, entry: LogEntry) {
        // Send to real-time subscribers
        let _ = self.logs_tx.send(entry.clone());
    }
}
