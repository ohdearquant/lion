use crate::orchestrator::metadata::{create_metadata, EventMetadata};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::SystemEvent;

/// Events related to task operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    /// Submit a new task
    Submitted {
        /// Task ID
        task_id: Uuid,
        /// Task payload
        payload: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// Task completed
    Completed {
        /// Task ID
        task_id: Uuid,
        /// Task result
        result: String,
        /// Event metadata
        metadata: EventMetadata,
    },
    /// Task error
    Error {
        /// Task ID
        task_id: Uuid,
        /// Error message
        error: String,
        /// Event metadata
        metadata: EventMetadata,
    },
}

impl TaskEvent {
    /// Create a new task submission event
    pub fn submit(task_id: Uuid, payload: impl Into<String>, correlation_id: Option<Uuid>) -> SystemEvent {
        SystemEvent::Task(TaskEvent::Submitted {
            task_id,
            payload: payload.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new task completion event
    pub fn complete(task_id: Uuid, result: impl Into<String>, correlation_id: Option<Uuid>) -> SystemEvent {
        SystemEvent::Task(TaskEvent::Completed {
            task_id,
            result: result.into(),
            metadata: create_metadata(correlation_id),
        })
    }

    /// Create a new task error event
    pub fn error(task_id: Uuid, error: impl Into<String>, correlation_id: Option<Uuid>) -> SystemEvent {
        SystemEvent::Task(TaskEvent::Error {
            task_id,
            error: error.into(),
            metadata: create_metadata(correlation_id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_events() {
        let task_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());

        // Test submit event
        match TaskEvent::submit(task_id, "test payload", correlation_id) {
            SystemEvent::Task(TaskEvent::Submitted {
                task_id: tid,
                payload,
                metadata,
            }) => {
                assert_eq!(tid, task_id);
                assert_eq!(payload, "test payload");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Submitted event"),
        }

        // Test complete event
        match TaskEvent::complete(task_id, "test result", correlation_id) {
            SystemEvent::Task(TaskEvent::Completed {
                task_id: tid,
                result,
                metadata,
            }) => {
                assert_eq!(tid, task_id);
                assert_eq!(result, "test result");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Completed event"),
        }

        // Test error event
        match TaskEvent::error(task_id, "test error", correlation_id) {
            SystemEvent::Task(TaskEvent::Error {
                task_id: tid,
                error,
                metadata,
            }) => {
                assert_eq!(tid, task_id);
                assert_eq!(error, "test error");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_serialization() {
        let task_id = Uuid::new_v4();
        let correlation_id = Some(Uuid::new_v4());
        let event = TaskEvent::Submitted {
            task_id,
            payload: "test payload".to_string(),
            metadata: create_metadata(correlation_id),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: TaskEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            TaskEvent::Submitted {
                task_id: tid,
                payload,
                metadata,
            } => {
                assert_eq!(tid, task_id);
                assert_eq!(payload, "test payload");
                assert_eq!(metadata.correlation_id, correlation_id);
            }
            _ => panic!("Expected Submitted event after deserialization"),
        }
    }
}