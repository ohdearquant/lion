use agentic_core::orchestrator::events::SystemEvent;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug)]
pub struct AppState {
    pub orchestrator_tx: mpsc::Sender<SystemEvent>,
    pub plugins: RwLock<HashMap<Uuid, PluginInfo>>,
    pub logs_tx: broadcast::Sender<String>,
}
