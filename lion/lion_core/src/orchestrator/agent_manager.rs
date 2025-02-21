use crate::{
    types::ParticipantState,
    events::sse::NetworkEventSender,
    orchestrator::OrchestratorError,
};
use tokio::sync::RwLock;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use tokio::time::Instant;
use tracing::{error, warn};

/// Manages agent state and concurrency
pub struct AgentManager {
    active_agents: Arc<RwLock<HashMap<Uuid, ParticipantState>>>,
    agent_timeouts: Arc<RwLock<HashMap<Uuid, Instant>>>,
    network_tx: NetworkEventSender,
    max_concurrent_agents: usize,
    agent_timeout: std::time::Duration,
}

impl AgentManager {
    pub fn new(
        max_concurrent_agents: usize,
        agent_timeout: std::time::Duration,
        network_tx: NetworkEventSender,
    ) -> Self {
        Self {
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            agent_timeouts: Arc::new(RwLock::new(HashMap::new())),
            network_tx,
            max_concurrent_agents,
            agent_timeout,
        }
    }

    /// Check if we can spawn a new agent
    pub async fn can_spawn_agent(&self) -> bool {
        let active_count = self.active_agents.read().await.len();
        active_count < self.max_concurrent_agents
    }

    /// Register a new agent
    pub async fn register_agent(&self, agent_id: Uuid) -> Result<(), OrchestratorError> {
        if !self.can_spawn_agent().await {
            return Err(OrchestratorError::SchedulingError(
                "Maximum concurrent agent limit reached".to_string()
            ));
        }

        self.active_agents.write().await.insert(agent_id, ParticipantState::Initializing);
        self.agent_timeouts.write().await.insert(agent_id, Instant::now());
        
        // Notify network
        self.network_tx.send_agent_status(agent_id, "spawned".to_string())
            .map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

        Ok(())
    }

    /// Remove an agent
    pub async fn remove_agent(&self, agent_id: Uuid, status: &str) -> Result<(), OrchestratorError> {
        self.active_agents.write().await.remove(&agent_id);
        self.agent_timeouts.write().await.remove(&agent_id);

        // Notify network about status change
        self.network_tx.send_agent_status(agent_id, status.to_string())
            .map_err(|e| OrchestratorError::ChannelError(e.to_string()))?;

        Ok(())
    }

    /// Update agent timeout
    pub async fn update_agent_timeout(&self, agent_id: &Uuid) {
        if let Some(timeout) = self.agent_timeouts.write().await.get_mut(agent_id) {
            *timeout = Instant::now();
        }
    }

    /// Monitor agent timeouts
    pub async fn monitor_timeouts(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            let mut timeouts = self.agent_timeouts.write().await;
            let mut active_agents = self.active_agents.write().await;
            
            let now = Instant::now();
            let mut timed_out = Vec::new();
            
            for (&agent_id, &last_active) in timeouts.iter() {
                if now.duration_since(last_active) > self.agent_timeout {
                    timed_out.push(agent_id);
                }
            }
            
            for agent_id in timed_out {
                warn!("Agent {} timed out", agent_id);
                timeouts.remove(&agent_id);
                active_agents.remove(&agent_id);
                
                // Notify about timeout
                if let Err(e) = self.network_tx.send_agent_status(agent_id, "timeout".to_string()) {
                    error!("Failed to send timeout notification: {}", e);
                }
            }
        }
    }

    /// Get active agent count
    pub async fn active_agent_count(&self) -> usize {
        self.active_agents.read().await.len()
    }
}