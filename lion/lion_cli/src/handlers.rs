use lion_core::orchestrator::{
    events::{AgentEvent, PluginEvent, SystemEvent, TaskEvent},
    metadata::EventMetadata,
    Orchestrator, OrchestratorConfig,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::logging::log_event;

pub fn handle_ci() {
    // Execute CI script
    let output = std::process::Command::new("sh")
        .arg("scripts/ci.sh")
        .output()
        .expect("Failed to execute CI script");

    println!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        std::process::exit(1);
    }
}

pub fn handle_test_cli() {
    // Execute test CLI script
    let output = std::process::Command::new("sh")
        .arg("scripts/test_cli.sh")
        .output()
        .expect("Failed to execute test CLI script");

    println!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        std::process::exit(1);
    }
}

pub async fn handle_spawn_agent(prompt: String, correlation_id: Option<String>) {
    let orchestrator = Orchestrator::new(OrchestratorConfig::default());
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Convert correlation_id string to UUID if provided
    let correlation_id = correlation_id
        .and_then(|id| Uuid::parse_str(&id).ok())
        .or_else(|| Some(Uuid::new_v4()));

    // Send agent spawn event
    let event = SystemEvent::Agent(AgentEvent::Spawned {
        agent_id: Uuid::new_v4(),
        prompt,
        metadata: EventMetadata::new(correlation_id),
    });

    log_event(&event);

    if let Err(e) = sender.send(event).await {
        error!("Failed to spawn agent: {}", e);
        return;
    }

    // Wait for completion events
    while let Ok(event) = completion_rx.recv().await {
        log_event(&event);
        if let Err(e) = handle_agent_completion(&event, &sender).await {
            error!("Error handling agent completion: {}", e);
        }
    }
}

pub async fn handle_demo(data: String, correlation_id: Option<String>) {
    let orchestrator = Orchestrator::new(OrchestratorConfig::default());
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Convert correlation_id string to UUID if provided
    let correlation_id = correlation_id
        .and_then(|id| Uuid::parse_str(&id).ok())
        .or_else(|| Some(Uuid::new_v4()));

    // Send task event
    let event = SystemEvent::Task(TaskEvent::Submitted {
        task_id: Uuid::new_v4(),
        payload: data,
        metadata: EventMetadata::new(correlation_id),
    });

    log_event(&event);

    if let Err(e) = sender.send(event).await {
        error!("Failed to submit task: {}", e);
        return;
    }

    // Wait for completion events
    while let Ok(event) = completion_rx.recv().await {
        log_event(&event);
        if let Err(e) = handle_task_completion(&event, &sender).await {
            error!("Error handling task completion: {}", e);
        }
    }
}

pub fn handle_load_plugin(manifest_path: String) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let orchestrator = Orchestrator::new(OrchestratorConfig::default());
        let sender = orchestrator.sender();
        let mut completion_rx = orchestrator.completion_receiver();

        // Spawn orchestrator
        tokio::spawn(orchestrator.run());

        // Send plugin load event
        let event = SystemEvent::Plugin(PluginEvent::Load {
            plugin_id: Uuid::new_v4(),
            manifest: toml::from_str(&tokio::fs::read_to_string(&manifest_path).await.unwrap())
                .unwrap(),
            manifest_path: Some(manifest_path),
            metadata: EventMetadata::new(None),
        });

        log_event(&event);

        if let Err(e) = sender.send(event).await {
            error!("Failed to load plugin: {}", e);
            return;
        }

        // Wait for completion events
        while let Ok(event) = completion_rx.recv().await {
            log_event(&event);
            if let Err(e) = handle_plugin_completion(&event, &sender).await {
                error!("Error handling plugin completion: {}", e);
            }
        }
    });
}

pub async fn handle_invoke_plugin(
    plugin_id: String,
    input: String,
    correlation_id: Option<String>,
) {
    let orchestrator = Orchestrator::new(OrchestratorConfig::default());
    let sender = orchestrator.sender();
    let mut completion_rx = orchestrator.completion_receiver();

    // Spawn orchestrator
    tokio::spawn(orchestrator.run());

    // Convert plugin_id string to UUID
    let plugin_id = match Uuid::parse_str(&plugin_id) {
        Ok(id) => id,
        Err(e) => {
            error!("Invalid plugin ID: {}", e);
            return;
        }
    };

    // Convert correlation_id string to UUID if provided
    let correlation_id = correlation_id
        .and_then(|id| Uuid::parse_str(&id).ok())
        .or_else(|| Some(Uuid::new_v4()));

    // Send plugin invocation event
    let event = SystemEvent::Plugin(PluginEvent::Invoked {
        plugin_id,
        input,
        metadata: EventMetadata::new(correlation_id),
    });

    log_event(&event);

    if let Err(e) = sender.send(event).await {
        error!("Failed to invoke plugin: {}", e);
        return;
    }

    // Wait for completion events
    while let Ok(event) = completion_rx.recv().await {
        log_event(&event);
        if let Err(e) = handle_plugin_completion(&event, &sender).await {
            error!("Error handling plugin completion: {}", e);
        }
    }
}

pub async fn handle_list_plugins(
    orchestrator: &mpsc::Sender<SystemEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let event = SystemEvent::Plugin(PluginEvent::List);
    log_event(&event);
    debug!("Sending list plugins event: {:?}", event);
    orchestrator.send(event).await?;
    Ok(())
}

pub async fn handle_agent_completion(
    event: &SystemEvent,
    _orchestrator: &mpsc::Sender<SystemEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        SystemEvent::Agent(AgentEvent::Completed {
            agent_id, result, ..
        }) => {
            info!(agent_id = %agent_id, "Agent completed with result: {}", result);
            Ok(())
        }
        SystemEvent::Agent(AgentEvent::Error {
            agent_id, error, ..
        }) => {
            error!(agent_id = %agent_id, "Agent error: {}", error);
            Ok(())
        }
        _ => Ok(()),
    }
}

pub async fn handle_task_completion(
    event: &SystemEvent,
    _orchestrator: &mpsc::Sender<SystemEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        SystemEvent::Task(TaskEvent::Completed {
            task_id, result, ..
        }) => {
            info!(task_id = %task_id, "Task completed with result: {}", result);
            Ok(())
        }
        SystemEvent::Task(TaskEvent::Error { task_id, error, .. }) => {
            error!(task_id = %task_id, "Task error: {}", error);
            Ok(())
        }
        _ => Ok(()),
    }
}

pub async fn handle_plugin_completion(
    event: &SystemEvent,
    _orchestrator: &mpsc::Sender<SystemEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        SystemEvent::Plugin(PluginEvent::Result {
            plugin_id, result, ..
        }) => {
            info!(plugin_id = %plugin_id, "Plugin completed with result: {}", result);
            Ok(())
        }
        SystemEvent::Plugin(PluginEvent::Error {
            plugin_id, error, ..
        }) => {
            error!(plugin_id = %plugin_id, "Plugin error: {}", error);
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_handle_list_plugins() {
        let (tx, mut rx) = mpsc::channel(1);
        handle_list_plugins(&tx).await.unwrap();

        let event = rx.recv().await.unwrap();
        match event {
            SystemEvent::Plugin(PluginEvent::List) => (),
            _ => panic!("Expected List event"),
        }
    }

    #[tokio::test]
    async fn test_handle_plugin_completion() {
        let (tx, _) = mpsc::channel(1);
        let plugin_id = Uuid::new_v4();

        // Test successful completion
        let event = SystemEvent::Plugin(PluginEvent::Result {
            plugin_id,
            result: "test result".into(),
            metadata: EventMetadata::new(None),
        });
        handle_plugin_completion(&event, &tx).await.unwrap();

        // Test error
        let event = SystemEvent::Plugin(PluginEvent::Error {
            plugin_id,
            error: "test error".into(),
            metadata: EventMetadata::new(None),
        });
        handle_plugin_completion(&event, &tx).await.unwrap();
    }

    #[tokio::test]
    async fn test_handle_task_completion() {
        let (tx, _) = mpsc::channel(1);
        let task_id = Uuid::new_v4();

        // Test successful completion
        let event = SystemEvent::Task(TaskEvent::Completed {
            task_id,
            result: "test result".into(),
            metadata: EventMetadata::new(None),
        });
        handle_task_completion(&event, &tx).await.unwrap();

        // Test error
        let event = SystemEvent::Task(TaskEvent::Error {
            task_id,
            error: "test error".into(),
            metadata: EventMetadata::new(None),
        });
        handle_task_completion(&event, &tx).await.unwrap();
    }

    #[tokio::test]
    async fn test_handle_agent_completion() {
        let (tx, _) = mpsc::channel(1);
        let agent_id = Uuid::new_v4();

        // Test successful completion
        let event = SystemEvent::Agent(AgentEvent::Completed {
            agent_id,
            result: "test result".into(),
            metadata: EventMetadata::new(None),
        });
        handle_agent_completion(&event, &tx).await.unwrap();

        // Test error
        let event = SystemEvent::Agent(AgentEvent::Error {
            agent_id,
            error: "test error".into(),
            metadata: EventMetadata::new(None),
        });
        handle_agent_completion(&event, &tx).await.unwrap();
    }
}
