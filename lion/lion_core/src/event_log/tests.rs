use super::*;
use crate::orchestrator::{
    events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
    metadata::EventMetadata,
};
use uuid::Uuid;

#[test]
fn test_event_log_basic_flow() {
    let log = EventLog::new();
    let task_id = Uuid::new_v4();
    let correlation_id = Some(Uuid::new_v4());

    // Submit task
    log.append(SystemEvent::Task(TaskEvent::Submitted {
        task_id,
        payload: "test task".into(),
        metadata: EventMetadata::new(correlation_id),
    }));

    // Complete task
    log.append(SystemEvent::Task(TaskEvent::Completed {
        task_id,
        result: "test result".into(),
        metadata: EventMetadata::new(correlation_id),
    }));

    // Verify events were logged
    let records = log.all();
    assert_eq!(records.len(), 2, "Should have logged 2 events");

    // Check first event is TaskSubmitted
    match &records[0].event {
        SystemEvent::Task(TaskEvent::Submitted {
            task_id: t,
            payload,
            metadata,
        }) => {
            assert_eq!(*t, task_id);
            assert_eq!(payload, "test task");
            assert_eq!(metadata.correlation_id, correlation_id);
        }
        _ => panic!("First event should be TaskSubmitted"),
    }

    // Check second event is TaskCompleted
    match &records[1].event {
        SystemEvent::Task(TaskEvent::Completed {
            task_id: t,
            result,
            metadata,
        }) => {
            assert_eq!(*t, task_id);
            assert_eq!(result, "test result");
            assert_eq!(metadata.correlation_id, correlation_id);
        }
        _ => panic!("Second event should be TaskCompleted"),
    }

    // Verify replay summary
    let summary = log.replay_summary();
    assert!(summary.contains("Total Events: 2"));
    assert!(summary.contains("Tasks Submitted: 1"));
    assert!(summary.contains("Tasks Completed: 1"));
    assert!(summary.contains("Tasks Failed: 0"));
    assert!(summary.contains(&task_id.to_string()));
}

#[test]
fn test_event_log_with_error() {
    let log = EventLog::new();
    let task_id = Uuid::new_v4();

    // Submit task
    log.append(SystemEvent::Task(TaskEvent::Submitted {
        task_id,
        payload: "test task".into(),
        metadata: EventMetadata::new(None),
    }));

    // Task fails
    log.append(SystemEvent::Task(TaskEvent::Error {
        task_id,
        error: "test error".into(),
        metadata: EventMetadata::new(None),
    }));

    let summary = log.replay_summary();
    assert!(summary.contains("Tasks Failed: 1"));
    assert!(summary.contains("Failed with error: test error"));
}

#[test]
fn test_event_log_with_plugin() {
    let log = EventLog::new();
    let plugin_id = Uuid::new_v4();

    // Invoke plugin
    log.append(SystemEvent::Plugin(PluginEvent::Invoked {
        plugin_id,
        input: "test input".into(),
        metadata: EventMetadata::new(None),
    }));

    // Plugin completes
    log.append(SystemEvent::Plugin(PluginEvent::Result {
        plugin_id,
        result: "test output".into(),
        metadata: EventMetadata::new(None),
    }));

    let summary = log.replay_summary();
    assert!(summary.contains("Plugins Invoked: 1"));
    assert!(summary.contains("Plugins Completed: 1"));
    assert!(summary.contains("test output"));
}

#[test]
fn test_event_log_with_agent() {
    let log = EventLog::new();
    let agent_id = Uuid::new_v4();

    // Spawn agent
    log.append(SystemEvent::Agent(AgentEvent::Spawned {
        agent_id,
        prompt: "test prompt".into(),
        metadata: EventMetadata::new(None),
    }));

    // Agent produces partial output
    log.append(SystemEvent::Agent(AgentEvent::PartialOutput {
        agent_id,
        output: "partial result".into(),
        metadata: EventMetadata::new(None),
    }));

    // Agent completes
    log.append(SystemEvent::Agent(AgentEvent::Completed {
        agent_id,
        result: "final result".into(),
        metadata: EventMetadata::new(None),
    }));

    let summary = log.replay_summary();
    assert!(summary.contains("Agents Spawned: 1"));
    assert!(summary.contains("Agents Completed: 1"));
    assert!(summary.contains("partial result"));
    assert!(summary.contains("final result"));
}

#[test]
fn test_empty_event_log() {
    let log = EventLog::new();
    assert_eq!(log.replay_summary(), "No events to replay.");
}

#[test]
fn test_event_stats() {
    let mut stats = EventStats::new();
    let task_id = Uuid::new_v4();
    let plugin_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();

    // Process task events
    stats.process_event(&SystemEvent::Task(TaskEvent::Submitted {
        task_id,
        payload: "test".into(),
        metadata: EventMetadata::new(None),
    }));
    assert_eq!(stats.tasks_submitted, 1);

    // Process plugin events
    stats.process_event(&SystemEvent::Plugin(PluginEvent::Invoked {
        plugin_id,
        input: "test".into(),
        metadata: EventMetadata::new(None),
    }));
    assert_eq!(stats.plugins_invoked, 1);

    // Process agent events
    stats.process_event(&SystemEvent::Agent(AgentEvent::Spawned {
        agent_id,
        prompt: "test".into(),
        metadata: EventMetadata::new(None),
    }));
    assert_eq!(stats.agents_spawned, 1);
}
