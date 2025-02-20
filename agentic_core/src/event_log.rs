use crate::SystemEvent;
use std::collections::VecDeque;
use std::sync::Mutex;
use uuid::Uuid;

/// A thread-safe event log that stores system events
pub struct EventLog {
    events: Mutex<VecDeque<SystemEvent>>,
    max_size: usize,
}

impl EventLog {
    /// Create a new event log with a maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            events: Mutex::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }

    /// Log a new event, removing oldest if at capacity
    pub fn log_event(&self, event: SystemEvent) {
        let mut events = self.events.lock().unwrap();
        if events.len() >= self.max_size {
            events.pop_front();
        }
        events.push_back(event);
    }

    /// Get all events
    pub fn get_events(&self) -> Vec<SystemEvent> {
        let events = self.events.lock().unwrap();
        events.iter().cloned().collect()
    }

    /// Get a summary of all events
    pub fn get_summary(&self) -> Vec<String> {
        let events = self.events.lock().unwrap();
        events.iter().map(|e| format!("{:?}", e)).collect()
    }

    /// Get events related to a specific agent
    pub fn get_agent_events(&self, agent_id: Uuid) -> Vec<SystemEvent> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .filter(|e| match e {
                SystemEvent::AgentSpawned { agent_id: id, .. }
                | SystemEvent::AgentPartialOutput { agent_id: id, .. }
                | SystemEvent::AgentCompleted { agent_id: id, .. }
                | SystemEvent::AgentError { agent_id: id, .. } => *id == agent_id,
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// Get a summary of events for a specific agent
    pub fn get_agent_summary(&self, agent_id: Uuid) -> Vec<String> {
        self.get_agent_events(agent_id)
            .iter()
            .map(|e| format!("{:?}", e))
            .collect()
    }

    /// Get events related to a specific plugin
    pub fn get_plugin_events(&self, plugin_id: Uuid) -> Vec<SystemEvent> {
        let events = self.events.lock().unwrap();
        events
            .iter()
            .filter(|e| match e {
                SystemEvent::PluginLoad { plugin_id: id, .. }
                | SystemEvent::PluginInvoked { plugin_id: id, .. }
                | SystemEvent::PluginResult { plugin_id: id, .. }
                | SystemEvent::PluginError { plugin_id: id, .. } => *id == plugin_id,
                _ => false,
            })
            .cloned()
            .collect()
    }

    /// Get a summary of events for a specific plugin
    pub fn get_plugin_summary(&self, plugin_id: Uuid) -> Vec<String> {
        self.get_plugin_events(plugin_id)
            .iter()
            .map(|e| format!("{:?}", e))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_log_capacity() {
        let log = EventLog::new(2);
        let id = Uuid::new_v4();

        // Add three events
        log.log_event(SystemEvent::TaskSubmitted {
            task_id: id,
            payload: "test1".to_string(),
        });
        log.log_event(SystemEvent::TaskSubmitted {
            task_id: id,
            payload: "test2".to_string(),
        });
        log.log_event(SystemEvent::TaskSubmitted {
            task_id: id,
            payload: "test3".to_string(),
        });

        // Should only contain last two events
        let events = log.get_events();
        assert_eq!(events.len(), 2);
        match &events[1] {
            SystemEvent::TaskSubmitted { payload, .. } => {
                assert_eq!(payload, "test3");
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[test]
    fn test_agent_events() {
        let log = EventLog::new(10);
        let agent_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();

        // Add mixed events
        log.log_event(SystemEvent::AgentSpawned {
            agent_id,
            prompt: None,
        });
        log.log_event(SystemEvent::AgentSpawned {
            agent_id: other_id,
            prompt: None,
        });
        log.log_event(SystemEvent::AgentCompleted {
            agent_id,
            result: "done".to_string(),
        });

        // Should only get events for specified agent
        let events = log.get_agent_events(agent_id);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_plugin_events() {
        let log = EventLog::new(10);
        let plugin_id = Uuid::new_v4();
        let manifest = crate::plugin_manager::PluginManifest {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "Test plugin".to_string(),
            entry_point: "test".to_string(),
            permissions: vec![],
            driver: None,
            functions: std::collections::HashMap::new(),
        };

        // Add plugin events
        log.log_event(SystemEvent::PluginLoad {
            plugin_id,
            manifest: manifest.clone(),
            manifest_path: Some("test/manifest.toml".to_string()),
        });
        log.log_event(SystemEvent::PluginResult {
            plugin_id,
            result: "success".to_string(),
        });

        // Should get both plugin events
        let events = log.get_plugin_events(plugin_id);
        assert_eq!(events.len(), 2);
    }
}
