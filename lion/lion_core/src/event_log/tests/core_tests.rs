use super::helpers::*;
use crate::event_log::EventLog;
use crate::SystemEvent;
use uuid::Uuid;

#[test]
fn test_event_log_basic_flow() {
    let log = EventLog::new();
    let task_id = Uuid::new_v4();
    let correlation_id = Some(Uuid::new_v4());

    // Create and append test events
    let events = create_test_task_events(task_id, correlation_id);
    for event in events {
        log.append(event);
    }

    // Verify events were logged
    let records = log.all();
    assert_eq!(records.len(), 2, "Should have logged 2 events");

    // Check first event is TaskSubmitted
    match &records[0].event {
        SystemEvent::TaskSubmitted {
            task_id: t,
            payload,
            metadata,
            ..
        } => {
            assert_eq!(*t, task_id);
            assert_eq!(payload, "test task");
            assert_eq!(metadata.correlation_id, correlation_id);
        }
        _ => panic!("First event should be TaskSubmitted"),
    }

    // Check second event is TaskCompleted
    match &records[1].event {
        SystemEvent::TaskCompleted {
            task_id: t,
            result,
            metadata,
            ..
        } => {
            assert_eq!(*t, task_id);
            assert_eq!(result, "test result");
            assert_eq!(metadata.correlation_id, correlation_id);
        }
        _ => panic!("Second event should be TaskCompleted"),
    }
}

#[test]
fn test_event_log_empty() {
    let log = EventLog::new();
    let records = log.all();
    assert!(records.is_empty(), "New event log should be empty");
}

#[test]
fn test_event_log_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let log = Arc::new(EventLog::new());
    let mut handles = vec![];

    // Spawn multiple threads to append events
    for _ in 0..10 {
        let log_clone = Arc::clone(&log);
        let handle = thread::spawn(move || {
            let task_id = Uuid::new_v4();
            let events = create_test_task_events(task_id, None);
            for event in events {
                log_clone.append(event);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all events were logged
    let records = log.all();
    assert_eq!(
        records.len(),
        20,
        "Should have logged 20 events (2 per thread)"
    );
}
