use super::events::AgentEvent;
use crate::types::agent::AgentState;
use async_trait::async_trait;
use std::{error::Error, fmt};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::protocol::{AgentProtocol, StreamingAgent};

/// Error type for mock agent operations
#[derive(Debug)]
pub struct MockError(String);

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for MockError {}

/// A mock agent that simulates streaming output
#[derive(Debug)]
pub struct MockStreamingAgent {
    id: Uuid,
    status: AgentState,
    chunks: Vec<String>,
    current_chunk: usize,
    event_tx: Option<mpsc::Sender<AgentEvent>>,
}

impl MockStreamingAgent {
    /// Create a new mock streaming agent
    pub fn new(chunks: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: AgentState::Initializing,
            chunks,
            current_chunk: 0,
            event_tx: None,
        }
    }

    /// Set the event sender
    pub fn with_event_sender(mut self, tx: mpsc::Sender<AgentEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Send an event if a sender is configured
    async fn send_event(&self, event: AgentEvent) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

#[async_trait]
impl AgentProtocol for MockStreamingAgent {
    type Input = String;
    type Output = String;
    type Error = MockError;

    fn id(&self) -> Uuid {
        self.id
    }

    fn status(&self) -> AgentState {
        self.status.clone()
    }

    async fn initialize(&mut self) -> Result<(), Self::Error> {
        self.status = AgentState::Running;
        self.send_event(AgentEvent::start(self.id, "Mock agent initialized"))
            .await;
        Ok(())
    }

    async fn process(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        if input.is_empty() {
            let error = MockError("Empty input".to_string());
            self.status = AgentState::Error;
            self.send_event(AgentEvent::error(self.id, error.to_string()))
                .await;
            return Err(error);
        }

        let mut output = String::new();
        while let Some(chunk) = self.next_chunk().await? {
            output.push_str(&chunk);
        }

        self.status = AgentState::Ready;
        self.send_event(AgentEvent::done(self.id, output.clone()))
            .await;
        Ok(output)
    }

    async fn stream_output(&mut self) -> Result<Option<String>, Self::Error> {
        self.next_chunk().await
    }

    async fn cleanup(&mut self) -> Result<(), Self::Error> {
        self.status = AgentState::Ready;
        Ok(())
    }
}

#[async_trait]
impl StreamingAgent for MockStreamingAgent {
    async fn next_chunk(&mut self) -> Result<Option<String>, Self::Error> {
        if self.current_chunk >= self.chunks.len() {
            return Ok(None);
        }

        let chunk = self.chunks[self.current_chunk].clone();
        self.current_chunk += 1;

        self.send_event(AgentEvent::partial_output(self.id, chunk.clone()))
            .await;

        Ok(Some(chunk))
    }

    fn has_more(&self) -> bool {
        self.current_chunk < self.chunks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_mock_streaming_agent() {
        let original_chunks = vec!["Hello".to_string(), ", ".to_string(), "world!".to_string()];
        let (tx, mut rx) = mpsc::channel(10);
        let chunks = original_chunks.clone();
        let mut agent = MockStreamingAgent::new(chunks).with_event_sender(tx);

        // Test initialization
        agent.initialize().await.unwrap();
        assert_eq!(agent.status(), AgentState::Running);

        // Verify initialization event
        if let Some(AgentEvent::Start { agent_id, .. }) = rx.recv().await {
            assert_eq!(agent_id, agent.id());
        } else {
            panic!("Expected Start event");
        }

        // Test streaming
        let result = agent.process("test input".to_string()).await.unwrap();
        assert_eq!(result, "Hello, world!");
        assert_eq!(agent.status(), AgentState::Ready);

        // Verify streaming events
        let mut received_chunks = Vec::new();
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::PartialOutput { output, .. } => received_chunks.push(output),
                AgentEvent::Done { final_output, .. } => {
                    assert_eq!(final_output, "Hello, world!");
                    break;
                }
                _ => continue,
            }
        }
        assert_eq!(received_chunks, vec!["Hello", ", ", "world!"]);

        // Test error handling
        let mut agent = MockStreamingAgent::new(original_chunks);
        let error = agent.process("".to_string()).await.unwrap_err();
        assert_eq!(error.to_string(), "Empty input");
        assert_eq!(agent.status(), AgentState::Error);
    }
}
