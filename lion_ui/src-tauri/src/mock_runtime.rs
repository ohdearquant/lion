use std::sync::Arc;
use tokio::sync::Mutex;

// Mock implementation of Runtime for testing
pub struct Runtime {
    agent_count: Arc<Mutex<usize>>,
}

impl Runtime {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            agent_count: Arc::new(Mutex::new(3)), // Default to 3 agents for testing
        })
    }

    pub fn get_agent_count(&self) -> Result<usize, String> {
        let agent_count = self.agent_count.blocking_lock();
        Ok(*agent_count)
    }
}
