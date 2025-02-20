use agentic_core::{
    orchestrator::{
        events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
        metadata::EventMetadata,
        Orchestrator,
    },
    plugin_manager::PluginManifest,
};
use serde_json::json;
use std::time::Duration;
use tokio;
use uuid::Uuid;

#[tokio::test]
async fn test_orchestrator_task_flow() {
    // Create orchestrator
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Submit task
    let task_id = Uuid::new_v4();
    let event = SystemEvent::Task(TaskEvent::Submitted {
        task_id,
        payload: "test task".into(),
        metadata: EventMetadata::new(None),
    });

    sender.send(event).await.unwrap();

    // Wait for completion
    if let Ok(SystemEvent::Task(TaskEvent::Completed {
        task_id: completed_id,
        ..
    })) = completion_rx.recv().await
    {
        assert_eq!(completed_id, task_id);
    } else {
        panic!("Expected task completion");
    }
}

#[tokio::test]
async fn test_orchestrator_plugin_flow() {
    // Create orchestrator
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Create test manifest
    let manifest = PluginManifest::new(
        "test-plugin".to_string(),
        "1.0.0".to_string(),
        "A test plugin".to_string(),
    );

    // Load plugin
    let plugin_id = Uuid::new_v4();
    let load_event = SystemEvent::Plugin(PluginEvent::Load {
        plugin_id,
        manifest,
        manifest_path: None,
        metadata: EventMetadata::new(None),
    });

    sender.send(load_event).await.unwrap();

    // Wait for load completion
    if let Ok(SystemEvent::Plugin(PluginEvent::Result {
        plugin_id: loaded_id,
        ..
    })) = completion_rx.recv().await
    {
        assert_eq!(loaded_id, plugin_id);
    } else {
        panic!("Expected plugin load completion");
    }

    // Invoke plugin
    let invoke_event = SystemEvent::Plugin(PluginEvent::Invoked {
        plugin_id,
        input: "test input".into(),
        metadata: EventMetadata::new(None),
    });

    sender.send(invoke_event).await.unwrap();

    // Wait for invocation completion
    if let Ok(SystemEvent::Plugin(PluginEvent::Result {
        plugin_id: invoked_id,
        ..
    })) = completion_rx.recv().await
    {
        assert_eq!(invoked_id, plugin_id);
    } else {
        panic!("Expected plugin invocation completion");
    }
}

#[tokio::test]
async fn test_orchestrator_agent_flow() {
    // Create orchestrator
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Spawn agent
    let agent_id = Uuid::new_v4();
    let event = SystemEvent::Agent(AgentEvent::Spawned {
        agent_id,
        prompt: "test prompt".into(),
        metadata: EventMetadata::new(None),
    });

    sender.send(event).await.unwrap();

    // Wait for completion
    if let Ok(SystemEvent::Agent(AgentEvent::Completed {
        agent_id: completed_id,
        ..
    })) = completion_rx.recv().await
    {
        assert_eq!(completed_id, agent_id);
    } else {
        panic!("Expected agent completion");
    }
}

#[tokio::test]
async fn test_orchestrator_concurrent_events() {
    // Create orchestrator
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Submit multiple events concurrently
    let mut event_ids = Vec::new();
    for i in 0..10 {
        let id = Uuid::new_v4();
        event_ids.push(id);

        let event = match i % 3 {
            0 => SystemEvent::Task(TaskEvent::Submitted {
                task_id: id,
                payload: format!("task {}", i),
                metadata: EventMetadata::new(None),
            }),
            1 => SystemEvent::Agent(AgentEvent::Spawned {
                agent_id: id,
                prompt: format!("agent {}", i),
                metadata: EventMetadata::new(None),
            }),
            _ => SystemEvent::Plugin(PluginEvent::Invoked {
                plugin_id: id,
                input: format!("plugin {}", i),
                metadata: EventMetadata::new(None),
            }),
        };

        sender.send(event).await.unwrap();
    }

    // Wait for all completions
    let mut completed = 0;
    while completed < event_ids.len() {
        if let Ok(event) = completion_rx.recv().await {
            match event {
                SystemEvent::Task(TaskEvent::Completed { task_id, .. })
                | SystemEvent::Agent(AgentEvent::Completed {
                    agent_id: task_id, ..
                })
                | SystemEvent::Plugin(PluginEvent::Result {
                    plugin_id: task_id, ..
                }) => {
                    assert!(event_ids.contains(&task_id));
                    completed += 1;
                }
                _ => {}
            }
        }
    }
}

#[tokio::test]
async fn test_orchestrator_error_handling() {
    // Create orchestrator
    let orchestrator = Orchestrator::new(100);
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Submit invalid task
    let task_id = Uuid::new_v4();
    let event = SystemEvent::Task(TaskEvent::Submitted {
        task_id,
        payload: "".into(), // Empty payload should trigger error
        metadata: EventMetadata::new(None),
    });

    sender.send(event).await.unwrap();

    // Wait for error
    if let Ok(SystemEvent::Task(TaskEvent::Error {
        task_id: error_id, ..
    })) = completion_rx.recv().await
    {
        assert_eq!(error_id, task_id);
    } else {
        panic!("Expected task error");
    }
}
