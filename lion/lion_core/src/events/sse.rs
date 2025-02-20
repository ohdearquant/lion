use futures::{Stream, StreamExt};
use std::{convert::Infallible, time::Duration};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use thiserror::Error;

use crate::types::traits::{LanguageMessage, LanguageMessageType};

/// Errors that can occur in SSE operations
#[derive(Error, Debug)]
pub enum SseError {
    #[error("Channel send error: {0}")]
    SendError(String),
    #[error("Channel receive error: {0}")]
    ReceiveError(String),
    #[error("Invalid event type: {0}")]
    InvalidEventType(String),
}

/// Types of SSE events in the language network
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum NetworkEvent {
    /// A language message event
    Message(LanguageMessage),
    /// A partial output from an agent
    PartialOutput {
        agent_id: Uuid,
        content: String,
        message_id: Uuid,
        sequence: u32,
    },
    /// An agent status update
    AgentStatus {
        agent_id: Uuid,
        status: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// A plugin execution event
    PluginEvent {
        plugin_id: Uuid,
        event_type: String,
        data: serde_json::Value,
    },
    /// A system event
    System {
        event_type: String,
        message: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Keep-alive event
    KeepAlive,
}

/// Configuration for SSE keep-alive settings
#[derive(Debug, Clone)]
pub struct SseKeepAlive {
    /// Interval between keep-alive messages
    pub interval: Duration,
    /// Text to send as keep-alive message
    pub text: String,
}

impl Default for SseKeepAlive {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(15),
            text: "keep-alive".to_string(),
        }
    }
}

/// A Server-Sent Events stream with configurable keep-alive
/// and support for multi-agent language network events
#[derive(Debug)]
pub struct EventStream<T> {
    /// The underlying stream of events
    pub stream: Box<dyn Stream<Item = Result<T, Infallible>> + Send>,
    /// Keep-alive configuration
    pub keep_alive: Option<SseKeepAlive>,
    /// Agent ID filter (if set, only receive events for this agent)
    pub agent_filter: Option<Uuid>,
    /// Event type filter
    pub event_filter: Option<Vec<String>>,
}

impl<T> EventStream<T> {
    /// Create a new event stream from a broadcast receiver
    pub fn new(rx: broadcast::Receiver<T>) -> Self
    where
        T: Clone + Send + 'static,
    {
        let stream = BroadcastStream::new(rx).map(|msg| {
            Ok::<_, Infallible>(msg.unwrap_or_else(|e| panic!("Error receiving message: {}", e)))
        });

        Self {
            stream: Box::new(stream),
            keep_alive: Some(SseKeepAlive::default()),
            agent_filter: None,
            event_filter: None,
        }
    }

    /// Set the keep-alive configuration
    pub fn keep_alive(mut self, keep_alive: Option<SseKeepAlive>) -> Self {
        self.keep_alive = keep_alive;
        self
    }

    /// Set the keep-alive interval
    pub fn keep_alive_interval(mut self, interval: Duration) -> Self {
        if let Some(keep_alive) = &mut self.keep_alive {
            keep_alive.interval = interval;
        } else {
            self.keep_alive = Some(SseKeepAlive {
                interval,
                ..Default::default()
            });
        }
        self
    }

    /// Set the keep-alive text
    pub fn keep_alive_text(mut self, text: impl Into<String>) -> Self {
        if let Some(keep_alive) = &mut self.keep_alive {
            keep_alive.text = text.into();
        } else {
            self.keep_alive = Some(SseKeepAlive {
                text: text.into(),
                ..Default::default()
            });
        }
        self
    }

    /// Filter events by agent ID
    pub fn filter_by_agent(mut self, agent_id: Uuid) -> Self {
        self.agent_filter = Some(agent_id);
        self
    }

    /// Filter events by type
    pub fn filter_by_type(mut self, event_types: Vec<String>) -> Self {
        self.event_filter = Some(event_types);
        self
    }
}

/// A sender for network events that supports partial outputs
#[derive(Debug, Clone)]
pub struct NetworkEventSender {
    tx: broadcast::Sender<NetworkEvent>,
}

impl NetworkEventSender {
    /// Create a new network event sender
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Send a language message
    pub fn send_message(&self, message: LanguageMessage) -> Result<(), SseError> {
        self.tx
            .send(NetworkEvent::Message(message))
            .map_err(|e| SseError::SendError(e.to_string()))
    }

    /// Send a partial output
    pub fn send_partial_output(
        &self,
        agent_id: Uuid,
        content: String,
        message_id: Uuid,
        sequence: u32,
    ) -> Result<(), SseError> {
        self.tx
            .send(NetworkEvent::PartialOutput {
                agent_id,
                content,
                message_id,
                sequence,
            })
            .map_err(|e| SseError::SendError(e.to_string()))
    }

    /// Send an agent status update
    pub fn send_agent_status(
        &self,
        agent_id: Uuid,
        status: String,
    ) -> Result<(), SseError> {
        self.tx
            .send(NetworkEvent::AgentStatus {
                agent_id,
                status,
                timestamp: chrono::Utc::now(),
            })
            .map_err(|e| SseError::SendError(e.to_string()))
    }

    /// Send a plugin event
    pub fn send_plugin_event(
        &self,
        plugin_id: Uuid,
        event_type: String,
        data: serde_json::Value,
    ) -> Result<(), SseError> {
        self.tx
            .send(NetworkEvent::PluginEvent {
                plugin_id,
                event_type,
                data,
            })
            .map_err(|e| SseError::SendError(e.to_string()))
    }

    /// Send a system event
    pub fn send_system_event(
        &self,
        event_type: String,
        message: String,
    ) -> Result<(), SseError> {
        self.tx
            .send(NetworkEvent::System {
                event_type,
                message,
                timestamp: chrono::Utc::now(),
            })
            .map_err(|e| SseError::SendError(e.to_string()))
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.tx.subscribe()
    }
}

impl Default for NetworkEventSender {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_event_stream() {
        let (tx, rx) = broadcast::channel(100);
        let stream = EventStream::new(rx);

        assert!(stream.keep_alive.is_some());
        let keep_alive = stream.keep_alive.unwrap();
        assert_eq!(keep_alive.interval, Duration::from_secs(15));
        assert_eq!(keep_alive.text, "keep-alive");

        // Test keep-alive configuration
        let stream = stream
            .keep_alive_interval(Duration::from_secs(30))
            .keep_alive_text("ping");

        let keep_alive = stream.keep_alive.unwrap();
        assert_eq!(keep_alive.interval, Duration::from_secs(30));
        assert_eq!(keep_alive.text, "ping");

        // Test disabling keep-alive
        let stream = stream.keep_alive(None);
        assert!(stream.keep_alive.is_none());

        // Clean up
        drop(tx);
    }

    #[tokio::test]
    async fn test_network_event_sender() {
        let sender = NetworkEventSender::new(100);
        let mut rx = sender.subscribe();

        // Test sending a message
        let message = LanguageMessage {
            id: Uuid::new_v4(),
            content: "test".to_string(),
            sender_id: Uuid::new_v4(),
            recipient_ids: vec![Uuid::new_v4()].into_iter().collect(),
            message_type: LanguageMessageType::Text,
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        };

        sender.send_message(message.clone()).unwrap();

        if let NetworkEvent::Message(received) = rx.recv().await.unwrap() {
            assert_eq!(received.id, message.id);
            assert_eq!(received.content, message.content);
        } else {
            panic!("Unexpected event type");
        }

        // Test sending partial output
        let agent_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        sender
            .send_partial_output(agent_id, "partial".to_string(), message_id, 1)
            .unwrap();

        if let NetworkEvent::PartialOutput {
            agent_id: received_agent_id,
            content,
            message_id: received_message_id,
            sequence,
        } = rx.recv().await.unwrap()
        {
            assert_eq!(received_agent_id, agent_id);
            assert_eq!(content, "partial");
            assert_eq!(received_message_id, message_id);
            assert_eq!(sequence, 1);
        } else {
            panic!("Unexpected event type");
        }
    }
}