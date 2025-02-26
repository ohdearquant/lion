//! Integration tests for event handling and multi-agent interactions.
//!
//! These tests verify the event handling mechanisms in the Lion microkernel,
//! focusing on how events flow between different components and how
//! agents interact through message passing.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use lion_core::error::{ConcurrencyError, Error, Result};
use lion_core::id::{MessageId, PluginId};
use lion_core::traits::{Capability, ConcurrencyManager};
use lion_core::types::{AccessRequest, Workflow, WorkflowNode};

/// A message in the event system.
#[derive(Debug, Clone)]
struct EventMessage {
    id: MessageId,
    source: PluginId,
    destination: Option<PluginId>,
    event_type: String,
    payload: Vec<u8>,
    timestamp: std::time::SystemTime,
}

impl EventMessage {
    fn new(source: PluginId, event_type: &str, payload: Vec<u8>) -> Self {
        Self {
            id: MessageId::new(),
            source,
            destination: None,
            event_type: event_type.to_string(),
            payload,
            timestamp: std::time::SystemTime::now(),
        }
    }

    fn with_destination(mut self, destination: PluginId) -> Self {
        self.destination = Some(destination);
        self
    }
}

/// A simple event bus that routes messages between agents.
struct EventBus {
    event_queues: Arc<Mutex<HashMap<PluginId, VecDeque<EventMessage>>>>,
    subscribers: Arc<Mutex<HashMap<String, Vec<PluginId>>>>,
    global_listeners: Arc<Mutex<Vec<PluginId>>>,
    processed_events: Arc<Mutex<Vec<EventMessage>>>,
}

impl EventBus {
    fn new() -> Self {
        Self {
            event_queues: Arc::new(Mutex::new(HashMap::new())),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
            global_listeners: Arc::new(Mutex::new(Vec::new())),
            processed_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a plugin with the event bus.
    fn register(&self, plugin_id: PluginId) {
        self.event_queues
            .lock()
            .unwrap()
            .insert(plugin_id, VecDeque::new());
    }

    /// Unregister a plugin from the event bus.
    fn unregister(&self, plugin_id: &PluginId) {
        self.event_queues.lock().unwrap().remove(plugin_id);

        // Remove from subscribers
        let mut subscribers = self.subscribers.lock().unwrap();
        for subscribers_list in subscribers.values_mut() {
            subscribers_list.retain(|id| id != plugin_id);
        }

        // Remove from global listeners
        let mut globals = self.global_listeners.lock().unwrap();
        globals.retain(|id| id != plugin_id);
    }

    /// Subscribe to a specific event type.
    fn subscribe(&self, plugin_id: PluginId, event_type: &str) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(plugin_id);
    }

    /// Subscribe to all events.
    fn subscribe_all(&self, plugin_id: PluginId) {
        self.global_listeners.lock().unwrap().push(plugin_id);
    }

    /// Publish an event to all subscribers.
    fn publish(&self, event: EventMessage) {
        let mut processed = self.processed_events.lock().unwrap();
        processed.push(event.clone());

        // Routing to specific destination
        if let Some(dest) = event.destination {
            let mut queues = self.event_queues.lock().unwrap();
            if let Some(queue) = queues.get_mut(&dest) {
                queue.push_back(event);
            }
            return;
        }

        // Routing to subscribers of this event type
        let subscribers = {
            let subs = self.subscribers.lock().unwrap();
            subs.get(&event.event_type).cloned().unwrap_or_default()
        };

        // Routing to global listeners
        let global_listeners = {
            let globals = self.global_listeners.lock().unwrap();
            globals.clone()
        };

        // Combine all recipients, excluding the source
        let mut recipients = subscribers;
        recipients.extend(global_listeners.iter().filter(|id| **id != event.source));
        // Sort recipients by their string representation
        recipients.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        recipients.dedup_by(|a, b| a.to_string() == b.to_string());

        // Add to each recipient's queue
        let mut queues = self.event_queues.lock().unwrap();
        for recipient in recipients {
            if let Some(queue) = queues.get_mut(&recipient) {
                queue.push_back(event.clone());
            }
        }
    }

    /// Get the next event for a plugin.
    fn get_next_event(&self, plugin_id: &PluginId) -> Option<EventMessage> {
        let mut queues = self.event_queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(plugin_id) {
            queue.pop_front()
        } else {
            None
        }
    }

    /// Get all processed events.
    fn get_processed_events(&self) -> Vec<EventMessage> {
        self.processed_events.lock().unwrap().clone()
    }
}

/// A test agent implementation that interacts with the event bus.
struct TestAgent {
    id: PluginId,
    event_bus: Arc<EventBus>,
    capabilities: Vec<Box<dyn Capability>>,
    concurrency_manager: Arc<dyn ConcurrencyManager>,
    state: Arc<Mutex<HashMap<String, String>>>,
    received_events: Arc<Mutex<Vec<EventMessage>>>,
}

impl TestAgent {
    fn new(
        id: PluginId,
        event_bus: Arc<EventBus>,
        concurrency_manager: Arc<dyn ConcurrencyManager>,
    ) -> Self {
        // Register with the event bus
        event_bus.register(id);

        Self {
            id,
            event_bus,
            capabilities: Vec::new(),
            concurrency_manager,
            state: Arc::new(Mutex::new(HashMap::new())),
            received_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a capability to the agent.
    fn add_capability(&mut self, capability: Box<dyn Capability>) {
        self.capabilities.push(capability);
    }

    /// Subscribe to a specific event type.
    fn subscribe(&self, event_type: &str) {
        self.event_bus.subscribe(self.id, event_type);
    }

    /// Subscribe to all events.
    fn subscribe_all(&self) {
        self.event_bus.subscribe_all(self.id);
    }

    /// Send an event.
    fn send_event(&self, event_type: &str, payload: Vec<u8>) -> Result<()> {
        let event = EventMessage::new(self.id, event_type, payload);
        self.event_bus.publish(event);
        Ok(())
    }

    /// Send an event to a specific agent.
    fn send_direct_event(
        &self,
        destination: PluginId,
        event_type: &str,
        payload: Vec<u8>,
    ) -> Result<()> {
        let event = EventMessage::new(self.id, event_type, payload).with_destination(destination);
        self.event_bus.publish(event);
        Ok(())
    }

    /// Process the next event in the queue.
    fn process_next_event(&self) -> Option<EventMessage> {
        if let Some(event) = self.event_bus.get_next_event(&self.id) {
            // Store the event in received_events
            self.received_events.lock().unwrap().push(event.clone());

            // Update state based on event
            let mut state = self.state.lock().unwrap();
            state.insert("last_event".to_string(), event.event_type.clone());
            state.insert("last_payload".to_string(), format!("{:?}", event.payload));

            Some(event)
        } else {
            None
        }
    }

    /// Get all received events.
    fn get_received_events(&self) -> Vec<EventMessage> {
        self.received_events.lock().unwrap().clone()
    }

    /// Get the current state.
    fn get_state(&self) -> HashMap<String, String> {
        self.state.lock().unwrap().clone()
    }

    /// Update the state.
    fn update_state(&self, key: &str, value: &str) {
        self.state
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
    }

    /// Call a function in another agent.
    fn call_function(&self, target: &PluginId, function: &str, params: &[u8]) -> Result<Vec<u8>> {
        // Check for capabilities
        for cap in &self.capabilities {
            let request = AccessRequest::plugin_call(target.to_string(), function.to_string());
            if cap.permits(&request).is_ok() {
                return self
                    .concurrency_manager
                    .call_function(target, function, params);
            }
        }

        // No appropriate capability found
        Err(Error::Capability(
            lion_core::error::CapabilityError::PermissionDenied(format!(
                "No capability to call function {} on plugin {}",
                function, target
            )),
        ))
    }
}

/// A test implementation of ConcurrencyManager for multi-agent tests.
struct TestConcurrencyManager {
    agents: Arc<Mutex<HashMap<PluginId, Arc<TestAgent>>>>,
}

impl TestConcurrencyManager {
    fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn register_agent(&self, agent: Arc<TestAgent>) {
        self.agents.lock().unwrap().insert(agent.id, agent.clone());
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ConcurrencyManager for TestConcurrencyManager {
    fn schedule_task(&self, task: Box<dyn FnOnce() + Send + 'static>) -> Result<()> {
        // Execute the task in a new thread
        thread::spawn(move || {
            task();
        });

        Ok(())
    }

    fn call_function(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
    ) -> Result<Vec<u8>> {
        let agents = self.agents.lock().unwrap();

        if let Some(agent) = agents.get(plugin_id) {
            // Simulate agent processing
            let mut response = function.as_bytes().to_vec();
            response.extend_from_slice(params);

            // Update agent state to record the call
            agent.update_state("last_function_called", function);
            agent.update_state("last_params", &format!("{:?}", params));

            Ok(response)
        } else {
            Err(ConcurrencyError::NoAvailableInstances(*plugin_id).into())
        }
    }

    fn call_function_with_timeout(
        &self,
        plugin_id: &PluginId,
        function: &str,
        params: &[u8],
        _timeout: Duration,
    ) -> Result<Vec<u8>> {
        // Simple implementation that ignores timeout for testing
        self.call_function(plugin_id, function, params)
    }
}

/// A test capability that allows plugin calls.
struct PluginCallCapability {
    allowed_plugins: Vec<String>,
}

impl PluginCallCapability {
    fn new(allowed_plugins: Vec<String>) -> Self {
        Self { allowed_plugins }
    }
}

impl Capability for PluginCallCapability {
    fn capability_type(&self) -> &str {
        "plugin_call"
    }

    fn permits(&self, request: &AccessRequest) -> lion_core::error::Result<()> {
        match request {
            AccessRequest::PluginCall {
                plugin_id,
                function: _,
            } => {
                if self.allowed_plugins.contains(plugin_id) {
                    Ok(())
                } else {
                    Err(Error::Capability(
                        lion_core::error::CapabilityError::PermissionDenied(format!(
                            "Plugin {} not in allowed list",
                            plugin_id
                        )),
                    ))
                }
            }
            _ => Err(Error::Capability(
                lion_core::error::CapabilityError::PermissionDenied(
                    "Only plugin calls are allowed".into(),
                ),
            )),
        }
    }
}

#[test]
fn test_event_publishing_and_subscribing() {
    let event_bus = Arc::new(EventBus::new());
    let concurrency_manager = Arc::new(TestConcurrencyManager::new());

    // Create two agents
    let agent1_id = PluginId::new();
    let agent1 = Arc::new(TestAgent::new(
        agent1_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let agent2_id = PluginId::new();
    let agent2 = Arc::new(TestAgent::new(
        agent2_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    // Agent 1 subscribes to "test_event"
    agent1.subscribe("test_event");

    // Agent 2 sends a test_event
    agent2.send_event("test_event", vec![1, 2, 3, 4]).unwrap();

    // Agent 1 should process the event
    let event = agent1.process_next_event();
    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, "test_event");
    assert_eq!(event.payload, vec![1, 2, 3, 4]);
    assert_eq!(event.source, agent2_id);

    // Agent 1's state should be updated
    let state = agent1.get_state();
    assert_eq!(state.get("last_event").unwrap(), "test_event");

    // Agent 2 should not have any events to process
    assert!(agent2.process_next_event().is_none());
}

#[test]
fn test_direct_messaging() {
    let event_bus = Arc::new(EventBus::new());
    let concurrency_manager = Arc::new(TestConcurrencyManager::new());

    // Create two agents
    let agent1_id = PluginId::new();
    let agent1 = Arc::new(TestAgent::new(
        agent1_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let agent2_id = PluginId::new();
    let agent2 = Arc::new(TestAgent::new(
        agent2_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    // Agent 1 sends a direct message to Agent 2
    agent1
        .send_direct_event(agent2_id, "direct_message", vec![5, 6, 7, 8])
        .unwrap();

    // Agent 2 should receive the direct message
    let event = agent2.process_next_event();
    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, "direct_message");
    assert_eq!(event.payload, vec![5, 6, 7, 8]);
    assert_eq!(event.source, agent1_id);

    // Agent 1 should not have received any events
    assert!(agent1.process_next_event().is_none());
}

#[test]
fn test_global_event_listening() {
    let event_bus = Arc::new(EventBus::new());
    let concurrency_manager = Arc::new(TestConcurrencyManager::new());

    // Create three agents
    let agent1_id = PluginId::new();
    let agent1 = Arc::new(TestAgent::new(
        agent1_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let agent2_id = PluginId::new();
    let agent2 = Arc::new(TestAgent::new(
        agent2_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let agent3_id = PluginId::new();
    let agent3 = Arc::new(TestAgent::new(
        agent3_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    // Agent 1 subscribes to all events
    agent1.subscribe_all();

    // Agent 2 sends one type of event
    agent2.send_event("type_a", vec![1, 2, 3]).unwrap();

    // Agent 3 sends another type of event
    agent3.send_event("type_b", vec![4, 5, 6]).unwrap();

    // Agent 1 should receive both events
    let received_events = {
        // Process all available events
        while agent1.process_next_event().is_some() {}
        agent1.get_received_events()
    };

    assert_eq!(received_events.len(), 2);
    assert!(received_events
        .iter()
        .any(|e| e.event_type == "type_a" && e.payload == vec![1, 2, 3]));
    assert!(received_events
        .iter()
        .any(|e| e.event_type == "type_b" && e.payload == vec![4, 5, 6]));
}

#[test]
fn test_capability_based_function_calling() {
    let event_bus = Arc::new(EventBus::new());
    let concurrency_manager = Arc::new(TestConcurrencyManager::new());

    // Create two agents
    let agent1_id = PluginId::new();
    let mut agent1 = TestAgent::new(agent1_id, event_bus.clone(), concurrency_manager.clone());

    let agent2_id = PluginId::new();
    let agent2 = Arc::new(TestAgent::new(
        agent2_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    // Register agent2 with the concurrency manager
    let test_cm = concurrency_manager
        .as_any()
        .downcast_ref::<TestConcurrencyManager>()
        .expect("Failed to downcast ConcurrencyManager");
    test_cm.register_agent(agent2.clone());

    // Add capability to agent1 allowing it to call agent2
    agent1.add_capability(Box::new(PluginCallCapability::new(vec![
        agent2_id.to_string()
    ])));
    let agent1 = Arc::new(agent1);

    // Agent 1 calls a function on Agent 2
    let result = agent1
        .call_function(&agent2_id, "test_function", &[9, 10, 11])
        .unwrap();

    // Check result
    let expected_result = {
        let mut result = "test_function".as_bytes().to_vec();
        result.extend_from_slice(&[9, 10, 11]);
        result
    };
    assert_eq!(result, expected_result);

    // Check that Agent 2's state was updated
    let state = agent2.get_state();
    assert_eq!(state.get("last_function_called").unwrap(), "test_function");
    assert_eq!(state.get("last_params").unwrap(), "[9, 10, 11]");
}

#[test]
fn test_workflow_event_propagation() {
    let event_bus = Arc::new(EventBus::new());
    let concurrency_manager = Arc::new(TestConcurrencyManager::new());

    // Create agents representing workflow nodes
    let load_id = PluginId::new();
    let load_agent = Arc::new(TestAgent::new(
        load_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let process_id = PluginId::new();
    let process_agent = Arc::new(TestAgent::new(
        process_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let visualize_id = PluginId::new();
    let visualize_agent = Arc::new(TestAgent::new(
        visualize_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    // Create a workflow
    let mut workflow = Workflow::new("Data Processing", "Process and analyze data");

    // Create nodes
    let load_node = WorkflowNode::new_plugin_call("Load Data", load_id.to_string(), "load_data");
    let load_node_id = load_node.id;
    workflow.add_node(load_node);

    let mut process_node =
        WorkflowNode::new_plugin_call("Process Data", process_id.to_string(), "process_data");
    process_node.add_dependency(load_node_id);
    let process_node_id = process_node.id;
    workflow.add_node(process_node);

    let mut visualize_node =
        WorkflowNode::new_plugin_call("Visualize Data", visualize_id.to_string(), "visualize");
    visualize_node.add_dependency(process_node_id);
    workflow.add_node(visualize_node);

    // Set up event subscriptions for workflow state changes
    load_agent.subscribe("node_complete");
    process_agent.subscribe("node_complete");
    visualize_agent.subscribe("node_complete");

    // Simulate workflow execution

    // Load node completes and publishes an event
    load_agent.update_state("state", "completed");
    load_agent
        .send_event("node_complete", load_node_id.to_bytes())
        .unwrap();

    // Process node sees the event, processes it, and publishes its own completion
    let event = process_agent.process_next_event().unwrap();
    assert_eq!(event.event_type, "node_complete");
    assert_eq!(event.source, load_id);

    process_agent.update_state("state", "completed");
    process_agent
        .send_event("node_complete", process_node_id.to_bytes())
        .unwrap();

    // Visualize node sees the process node completion event
    let event = visualize_agent.process_next_event().unwrap();
    assert_eq!(event.event_type, "node_complete");
    assert_eq!(event.source, process_id);

    // Verify workflow progression through events
    assert_eq!(process_agent.get_received_events().len(), 1);
    assert_eq!(visualize_agent.get_received_events().len(), 1);

    // The processed events in the event bus should show the chain of node completions
    let all_events = event_bus.get_processed_events();
    assert_eq!(all_events.len(), 2); // Two node_complete events
}

#[test]
fn test_event_ordering_and_delivery() {
    let event_bus = Arc::new(EventBus::new());
    let concurrency_manager = Arc::new(TestConcurrencyManager::new());

    // Create two agents
    let sender_id = PluginId::new();
    let sender = Arc::new(TestAgent::new(
        sender_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let receiver_id = PluginId::new();
    let receiver = Arc::new(TestAgent::new(
        receiver_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    // Receiver subscribes to test_event
    receiver.subscribe("test_event");

    // Send multiple events in sequence
    for i in 0..5 {
        sender.send_event("test_event", vec![i]).unwrap();
    }

    // Process all events and check ordering
    let mut received_payloads = Vec::new();
    while let Some(event) = receiver.process_next_event() {
        received_payloads.push(event.payload[0]);
    }

    // Verify events were received in order
    assert_eq!(received_payloads, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_agent_unregistration() {
    let event_bus = Arc::new(EventBus::new());
    let concurrency_manager = Arc::new(TestConcurrencyManager::new());

    // Create two agents
    let agent1_id = PluginId::new();
    let agent1 = Arc::new(TestAgent::new(
        agent1_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    let agent2_id = PluginId::new();
    let agent2 = Arc::new(TestAgent::new(
        agent2_id,
        event_bus.clone(),
        concurrency_manager.clone(),
    ));

    // Agent 2 subscribes to test_event
    agent2.subscribe("test_event");

    // Agent 1 sends an event
    agent1.send_event("test_event", vec![1, 2, 3]).unwrap();

    // Verify Agent 2 received it
    assert!(agent2.process_next_event().is_some());

    // Unregister Agent 2
    event_bus.unregister(&agent2_id);

    // Agent 1 sends another event
    agent1.send_event("test_event", vec![4, 5, 6]).unwrap();

    // Verify event bus processes the event but Agent 2 doesn't receive it
    let processed_events = event_bus.get_processed_events();
    assert_eq!(processed_events.len(), 2); // Both events were processed

    // Agent 2 should not have a queue anymore
    assert!(agent2.process_next_event().is_none());
}
