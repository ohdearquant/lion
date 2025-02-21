use crate::orchestrator::SystemEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

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
