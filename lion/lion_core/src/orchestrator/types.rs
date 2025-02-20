use crate::orchestrator::events::SystemEvent;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// A sender that can submit events to the orchestrator
pub type EventSender = mpsc::Sender<SystemEvent>;

/// A receiver that receives events from the orchestrator
pub type EventReceiver = mpsc::Receiver<SystemEvent>;

/// A sender for broadcasting completion events
pub type CompletionSender = broadcast::Sender<SystemEvent>;

/// A receiver for completion events
pub type CompletionReceiver = broadcast::Receiver<SystemEvent>;

/// Result type for orchestrator operations
pub type OrchestratorResult<T> = Result<T, OrchestratorError>;

/// Error types that can occur during orchestration
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    #[error("Agent not found: {0}")]
    AgentNotFound(Uuid),

    #[error("Plugin error: {0}")]
    PluginError(#[from] crate::plugin_manager::PluginError),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<mpsc::error::SendError<SystemEvent>> for OrchestratorError {
    fn from(err: mpsc::error::SendError<SystemEvent>) -> Self {
        OrchestratorError::ChannelError(err.to_string())
    }
}

impl From<broadcast::error::SendError<SystemEvent>> for OrchestratorError {
    fn from(err: broadcast::error::SendError<SystemEvent>) -> Self {
        OrchestratorError::ChannelError(err.to_string())
    }
}
