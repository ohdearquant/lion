use super::helpers::*;
use crate::event_log::EventLog;
use uuid::Uuid;

#[test]
fn test_empty_log_summary() {
    let log = EventLog::new();
    let summary = log.replay_summary();
    assert_eq!(summary, "No events to replay.");
}

#[test]
fn test_task_events_summary() {
    let log = EventLog::new();
    let task_id = Uuid::new_v4();

    // Create and append test task events
    let events = create_test_task_events(task_id, None);
    for event in events {
        log.append(event);
    }

    let summary = log.replay_summary();
    println!("Task Summary:\n{}", summary); // Debug output

    assert!(
        summary.contains("Total Events: 2"),
        "Missing total events count"
    );
    assert!(
        summary.contains("Tasks Submitted: 1"),
        "Missing tasks submitted count"
    );
    assert!(
        summary.contains("Tasks Completed: 1"),
        "Missing tasks completed count"
    );
    assert!(
        summary.contains("Tasks Failed: 0"),
        "Missing tasks failed count"
    );
    assert!(summary.contains(&task_id.to_string()), "Missing task ID");
    assert!(
        summary.contains("Submitted with payload: test task"),
        "Missing task submission details"
    );
    assert!(
        summary.contains("Completed with result: test result"),
        "Missing task completion details"
    );
}

#[test]
fn test_plugin_events_summary() {
    let log = EventLog::new();
    let plugin_id = Uuid::new_v4();

    // Create and append test plugin events
    let events = create_test_plugin_events(plugin_id);
    for event in events {
        log.append(event);
    }

    let summary = log.replay_summary();
    println!("Plugin Summary:\n{}", summary); // Debug output

    assert!(
        summary.contains("Plugins Invoked: 1"),
        "Missing plugins invoked count"
    );
    assert!(
        summary.contains("Plugins Completed: 1"),
        "Missing plugins completed count"
    );
    assert!(
        summary.contains("Plugins Failed: 0"),
        "Missing plugins failed count"
    );
    assert!(
        summary.contains(&plugin_id.to_string()),
        "Missing plugin ID"
    );
    assert!(
        summary.contains("Invoked with input: test input"),
        "Missing plugin input details"
    );
    assert!(
        summary.contains("Completed with output: test output"),
        "Missing plugin output details"
    );
}

#[test]
fn test_agent_events_summary() {
    let log = EventLog::new();
    let agent_id = Uuid::new_v4();

    // Create and append test agent events
    let events = create_test_agent_events(agent_id);
    for event in events {
        log.append(event);
    }

    let summary = log.replay_summary();
    println!("Agent Summary:\n{}", summary); // Debug output

    assert!(
        summary.contains("Agents Spawned: 1"),
        "Missing agents spawned count"
    );
    assert!(
        summary.contains("Agents Completed: 1"),
        "Missing agents completed count"
    );
    assert!(
        summary.contains("Agents Failed: 0"),
        "Missing agents failed count"
    );
    assert!(summary.contains(&agent_id.to_string()), "Missing agent ID");
    assert!(
        summary.contains("Spawned with prompt: test prompt"),
        "Missing agent spawn details"
    );
    assert!(
        summary.contains("Partial output: partial result"),
        "Missing agent partial output"
    );
    assert!(
        summary.contains("Completed with result: final result"),
        "Missing agent completion details"
    );
}

#[test]
fn test_mixed_events_summary() {
    let log = EventLog::new();

    // Add events for all types
    let task_id = Uuid::new_v4();
    let plugin_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();

    for event in create_test_task_events(task_id, None) {
        log.append(event);
    }
    for event in create_test_plugin_events(plugin_id) {
        log.append(event);
    }
    for event in create_test_agent_events(agent_id) {
        log.append(event);
    }

    let summary = log.replay_summary();
    println!("Mixed Summary:\n{}", summary); // Debug output

    // Verify all sections are present
    assert!(
        summary.contains("Task Statistics:"),
        "Missing task statistics section"
    );
    assert!(
        summary.contains("Plugin Statistics:"),
        "Missing plugin statistics section"
    );
    assert!(
        summary.contains("Agent Statistics:"),
        "Missing agent statistics section"
    );

    // Verify counts
    assert!(
        summary.contains("Tasks Submitted: 1"),
        "Missing tasks submitted count"
    );
    assert!(
        summary.contains("Plugins Invoked: 1"),
        "Missing plugins invoked count"
    );
    assert!(
        summary.contains("Agents Spawned: 1"),
        "Missing agents spawned count"
    );

    // Verify all IDs are present
    assert!(summary.contains(&task_id.to_string()), "Missing task ID");
    assert!(
        summary.contains(&plugin_id.to_string()),
        "Missing plugin ID"
    );
    assert!(summary.contains(&agent_id.to_string()), "Missing agent ID");
}
