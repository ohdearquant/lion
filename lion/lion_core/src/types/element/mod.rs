mod tests;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementData {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub metadata: Value,
}

impl ElementData {
    pub fn new(metadata: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            metadata,
        }
    }
}