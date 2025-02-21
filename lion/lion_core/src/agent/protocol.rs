use crate::types::agent::AgentState;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;
use uuid::Uuid;

/// Protocol for interacting with agents
#[async_trait]
pub trait AgentProtocol: Send + Sync {
    /// Type of input the agent accepts
    type Input: Serialize + DeserializeOwned + Send + Sync;
    /// Type of output the agent produces
    type Output: Serialize + DeserializeOwned + Send + Sync;
    /// Type of error the agent can return
    type Error: Error + Send + Sync;

    /// Get the agent's unique identifier
    fn id(&self) -> Uuid;

    /// Get the agent's current status
    fn status(&self) -> AgentState;

    /// Initialize the agent with any required configuration
    async fn initialize(&mut self) -> Result<(), Self::Error>;

    /// Process input and generate output
    async fn process(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error>;

    /// Get a stream of partial outputs while processing
    async fn stream_output(&mut self) -> Result<Option<String>, Self::Error>;

    /// Clean up any resources used by the agent
    async fn cleanup(&mut self) -> Result<(), Self::Error>;
}

/// A streaming agent that can provide partial outputs
#[async_trait]
pub trait StreamingAgent: AgentProtocol {
    /// Get the next chunk of output
    async fn next_chunk(&mut self) -> Result<Option<String>, Self::Error>;

    /// Check if the agent has more output available
    fn has_more(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct TestError(String);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl Error for TestError {}

    struct TestAgent {
        id: Uuid,
        status: AgentState,
    }

    #[async_trait]
    impl AgentProtocol for TestAgent {
        type Input = String;
        type Output = String;
        type Error = TestError;

        fn id(&self) -> Uuid {
            self.id
        }

        fn status(&self) -> AgentState {
            self.status.clone()
        }

        async fn initialize(&mut self) -> Result<(), Self::Error> {
            self.status = AgentState::Running;
            Ok(())
        }

        async fn process(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error> {
            if input.is_empty() {
                return Err(TestError("Empty input".to_string()));
            }
            Ok(format!("Processed: {}", input))
        }

        async fn stream_output(&mut self) -> Result<Option<String>, Self::Error> {
            Ok(Some("Partial output".to_string()))
        }

        async fn cleanup(&mut self) -> Result<(), Self::Error> {
            self.status = AgentState::Ready;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_agent_protocol() {
        let mut agent = TestAgent {
            id: Uuid::new_v4(),
            status: AgentState::Initializing,
        };

        // Test initialization
        agent.initialize().await.unwrap();
        assert_eq!(agent.status(), AgentState::Running);

        // Test processing
        let result = agent.process("test input".to_string()).await.unwrap();
        assert_eq!(result, "Processed: test input");

        // Test error handling
        let error = agent.process("".to_string()).await.unwrap_err();
        assert_eq!(error.to_string(), "Empty input");

        // Test streaming
        let output = agent.stream_output().await.unwrap();
        assert_eq!(output, Some("Partial output".to_string()));

        // Test cleanup
        agent.cleanup().await.unwrap();
        assert_eq!(agent.status(), AgentState::Ready);
    }
}
