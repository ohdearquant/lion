use crate::orchestrator::{EventMetadata, SystemEvent};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

pub(in crate::event_log::tests) fn create_test_metadata(
    correlation_id: Option<Uuid>,
) -> EventMetadata {
    EventMetadata {
        event_id: Uuid::new_v4(),
        timestamp: Utc::now(),
        correlation_id,
        context: json!({}),
    }
}

pub(in crate::event_log::tests) fn create_test_task_events(
    task_id: Uuid,
    correlation_id: Option<Uuid>,
) -> Vec<SystemEvent> {
    vec![
        SystemEvent::TaskSubmitted {
            task_id,
            payload: "test task".into(),
            metadata: create_test_metadata(correlation_id),
        },
        SystemEvent::TaskCompleted {
            task_id,
            result: "test result".into(),
            metadata: create_test_metadata(correlation_id),
        },
    ]
}

pub(in crate::event_log::tests) fn create_test_plugin_load_events(
    plugin_id: Uuid,
    manifest: String,
    success: bool,
) -> Vec<SystemEvent> {
    let mut events = vec![SystemEvent::PluginLoadRequested {
        plugin_id,
        manifest: manifest.clone(),
        metadata: create_test_metadata(None),
    }];

    if success {
        events.push(SystemEvent::PluginLoaded {
            plugin_id,
            name: "test_plugin".into(),
            version: "0.1.0".into(),
            description: "Test plugin".into(),
            metadata: create_test_metadata(None),
        });
    } else {
        events.push(SystemEvent::PluginError {
            plugin_id,
            error: "Failed to load plugin".into(),
            metadata: create_test_metadata(None),
        });
    }

    events
}

pub(in crate::event_log::tests) fn create_test_plugin_events(plugin_id: Uuid) -> Vec<SystemEvent> {
    vec![
        SystemEvent::PluginInvoked {
            plugin_id,
            input: "test input".into(),
            metadata: create_test_metadata(None),
        },
        SystemEvent::PluginResult {
            plugin_id,
            output: "test output".into(),
            metadata: create_test_metadata(None),
        },
    ]
}

pub(in crate::event_log::tests) fn create_test_agent_events(agent_id: Uuid) -> Vec<SystemEvent> {
    vec![
        SystemEvent::AgentSpawned {
            agent_id,
            prompt: "test prompt".into(),
            metadata: create_test_metadata(None),
        },
        SystemEvent::AgentPartialOutput {
            agent_id,
            chunk: "partial result".into(),
            metadata: create_test_metadata(None),
        },
        SystemEvent::AgentCompleted {
            agent_id,
            result: "final result".into(),
            metadata: create_test_metadata(None),
        },
    ]
}
