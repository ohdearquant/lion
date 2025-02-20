use crate::orchestrator::events::SystemEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub timestamp: DateTime<Utc>,
    pub event: SystemEvent,
}

impl fmt::Display for EventRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {:?}", self.timestamp, self.event)
    }
}

#[derive(Debug, Clone)]
pub struct EventLog {
    records: Arc<Mutex<Vec<EventRecord>>>,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn append(&self, event: SystemEvent) {
        let record = EventRecord {
            timestamp: Utc::now(),
            event,
        };
        if let Ok(mut records) = self.records.lock() {
            records.push(record);
        }
    }

    pub fn all(&self) -> Vec<EventRecord> {
        self.records
            .lock()
            .map(|records| records.clone())
            .unwrap_or_default()
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}
