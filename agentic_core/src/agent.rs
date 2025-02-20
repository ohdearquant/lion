use futures::Stream;
use std::pin::Pin;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Start {
        agent_id: Uuid,
        prompt: String,
    },
    PartialOutput {
        agent_id: Uuid,
        chunk: String,
    },
    Done {
        agent_id: Uuid,
        final_output: String,
    },
    Error {
        agent_id: Uuid,
        error: String,
    },
}

/// Protocol for implementing agent behavior
pub trait AgentProtocol {
    /// Handle an incoming event, possibly producing a new event in response
    fn on_event(&mut self, event: AgentEvent) -> Option<AgentEvent>;
}

/// A mock agent that simulates streaming responses
pub struct MockStreamingAgent {
    _id: Uuid, // Prefix with underscore to indicate intentionally unused
    chunks: Vec<String>,
}

impl MockStreamingAgent {
    pub fn new(id: Uuid) -> Self {
        Self {
            _id: id,
            chunks: Vec::new(),
        }
    }

    /// Creates a stream that yields mock chunks for demonstration
    pub fn stream_response(&self, prompt: &str) -> Pin<Box<dyn Stream<Item = String> + Send>> {
        let chunks = vec![
            format!("First chunk from prompt: {}", prompt),
            format!("Second chunk processing..."),
            format!("Final response complete."),
        ];
        Box::pin(tokio_stream::iter(chunks))
    }
}

impl AgentProtocol for MockStreamingAgent {
    fn on_event(&mut self, event: AgentEvent) -> Option<AgentEvent> {
        match event {
            AgentEvent::Start { agent_id, prompt } => {
                info!("Agent {} starting with prompt: {}", self._id, prompt);
                // In a real implementation, you might start an LLM call here
                // For now, just store the first chunk
                self.chunks.push(format!("Processing prompt: {}", prompt));
                Some(AgentEvent::PartialOutput {
                    agent_id,
                    chunk: self.chunks[0].clone(),
                })
            }
            AgentEvent::PartialOutput { agent_id, chunk } => {
                info!("Agent {} produced chunk: {}", self._id, chunk);
                self.chunks.push(chunk);
                // Simulate being done after a few chunks
                if self.chunks.len() >= 3 {
                    Some(AgentEvent::Done {
                        agent_id,
                        final_output: self.chunks.join("\n"),
                    })
                } else {
                    Some(AgentEvent::PartialOutput {
                        agent_id,
                        chunk: format!("Chunk {} for agent {}", self.chunks.len(), agent_id),
                    })
                }
            }
            AgentEvent::Done { .. } => {
                info!("Agent {} completed", self._id);
                None // No more events after Done
            }
            AgentEvent::Error { error, .. } => {
                info!("Agent {} encountered error: {}", self._id, error);
                None // No more events after Error
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    #[test]
    fn test_mock_agent_flow() {
        let agent_id = Uuid::new_v4();
        let mut agent = MockStreamingAgent::new(agent_id);

        // Start the agent
        let start_evt = AgentEvent::Start {
            agent_id,
            prompt: "test prompt".into(),
        };
        let mut current_evt = agent.on_event(start_evt);

        // Should get some partial outputs
        let mut chunks = Vec::new();
        while let Some(evt) = current_evt {
            match evt {
                AgentEvent::PartialOutput { chunk, .. } => {
                    chunks.push(chunk);
                    current_evt = agent.on_event(AgentEvent::PartialOutput {
                        agent_id,
                        chunk: chunks.last().unwrap().clone(),
                    });
                }
                AgentEvent::Done { final_output, .. } => {
                    assert!(final_output.contains("test prompt"));
                    current_evt = None;
                }
                _ => panic!("Unexpected event type"),
            }
        }

        // Should have produced some chunks before completing
        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_mock_streaming() {
        let agent_id = Uuid::new_v4();
        let agent = MockStreamingAgent::new(agent_id);
        let mut stream = agent.stream_response("test");

        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk);
        }

        assert_eq!(chunks.len(), 3); // Our mock produces 3 chunks
        assert!(chunks[0].contains("First chunk"));
        assert!(chunks[2].contains("Final response"));
    }
}
