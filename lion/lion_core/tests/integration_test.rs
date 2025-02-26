//! Integration tests for the Lion core library.
//!
//! These tests verify that the various components of the library work together
//! correctly. They focus on the interactions between different modules rather
//! than testing each module in isolation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use lion_core::error::{CapabilityError, Error, Result};
use lion_core::id::{CapabilityId, NodeId, PluginId, WorkflowId};
use lion_core::traits::{capability::Constraint, Capability};
use lion_core::types::{
    AccessRequest, ErrorPolicy, ExecutionOptions, MemoryRegion, MemoryRegionType, NodeType,
    PluginConfig, PluginMetadata, PluginState, ResourceUsage, Workflow, WorkflowNode,
};
use lion_core::utils::{config::Config, ConfigValue, LogLevel, Version};
use lion_core::{check_capability, log_event, with_capability};

/// A test file capability implementation.
struct FileCapability {
    path: PathBuf,
    read: bool,
    write: bool,
    execute: bool,
}

impl FileCapability {
    fn new(path: impl Into<PathBuf>, read: bool, write: bool, execute: bool) -> Self {
        Self {
            path: path.into(),
            read,
            write,
            execute,
        }
    }

    fn read_only(path: impl Into<PathBuf>) -> Self {
        Self::new(path, true, false, false)
    }

    fn write_only(path: impl Into<PathBuf>) -> Self {
        Self::new(path, false, true, false)
    }

    fn read_write(path: impl Into<PathBuf>) -> Self {
        Self::new(path, true, true, false)
    }
}

impl Capability for FileCapability {
    fn capability_type(&self) -> &str {
        "file"
    }

    fn permits(&self, request: &AccessRequest) -> Result<()> {
        match request {
            AccessRequest::File {
                path,
                read,
                write,
                execute,
            } => {
                // Check if the path is within our capability's path
                if !path.starts_with(&self.path) {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        format!(
                            "Path {} is not within allowed path {}",
                            path.display(),
                            self.path.display()
                        ),
                    )));
                }

                // Check if the requested operations are allowed
                if *read && !self.read {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        "Read not allowed".into(),
                    )));
                }

                if *write && !self.write {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        "Write not allowed".into(),
                    )));
                }

                if *execute && !self.execute {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        "Execute not allowed".into(),
                    )));
                }

                Ok(())
            }
            _ => Err(Error::Capability(CapabilityError::PermissionDenied(
                "File capability only permits file access".into(),
            ))),
        }
    }

    fn constrain(&self, constraints: &[Constraint]) -> Result<Box<dyn Capability>> {
        let mut new_cap = FileCapability {
            path: self.path.clone(),
            read: self.read,
            write: self.write,
            execute: self.execute,
        };

        for constraint in constraints {
            match constraint {
                Constraint::FilePath(path) => {
                    let new_path = PathBuf::from(path);
                    // Ensure new path is within the original path
                    if !new_path.starts_with(&self.path) {
                        return Err(Error::Capability(CapabilityError::ConstraintError(
                            format!(
                                "New path {} is not within original path {}",
                                new_path.display(),
                                self.path.display()
                            ),
                        )));
                    }
                    new_cap.path = new_path;
                }
                Constraint::FileOperation {
                    read,
                    write,
                    execute,
                } => {
                    // Can only remove permissions, not add them
                    new_cap.read = self.read && *read;
                    new_cap.write = self.write && *write;
                    new_cap.execute = self.execute && *execute;
                }
                _ => {
                    return Err(Error::Capability(CapabilityError::ConstraintError(
                        "Unsupported constraint for file capability".into(),
                    )))
                }
            }
        }

        Ok(Box::new(new_cap))
    }
}

/// A test network capability implementation.
struct NetworkCapability {
    host: String,
    port_range: (u16, u16),
    connect: bool,
    listen: bool,
}

impl NetworkCapability {
    fn new(host: impl Into<String>, port_range: (u16, u16), connect: bool, listen: bool) -> Self {
        Self {
            host: host.into(),
            port_range,
            connect,
            listen,
        }
    }

    fn outbound(host: impl Into<String>, port_range: (u16, u16)) -> Self {
        Self::new(host, port_range, true, false)
    }

    fn inbound(host: impl Into<String>, port_range: (u16, u16)) -> Self {
        Self::new(host, port_range, false, true)
    }
}

impl Capability for NetworkCapability {
    fn capability_type(&self) -> &str {
        "network"
    }

    fn permits(&self, request: &AccessRequest) -> Result<()> {
        match request {
            AccessRequest::Network {
                host,
                port,
                connect,
                listen,
            } => {
                // Check if the host is allowed
                if self.host != "*" && self.host != *host {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        format!("Host {} is not allowed", host),
                    )));
                }

                // Check if the port is in range
                if *port < self.port_range.0 || *port > self.port_range.1 {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        format!(
                            "Port {} is not in allowed range {}-{}",
                            port, self.port_range.0, self.port_range.1
                        ),
                    )));
                }

                // Check if the operation is allowed
                if *connect && !self.connect {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        "Connect not allowed".into(),
                    )));
                }

                if *listen && !self.listen {
                    return Err(Error::Capability(CapabilityError::PermissionDenied(
                        "Listen not allowed".into(),
                    )));
                }

                Ok(())
            }
            _ => Err(Error::Capability(CapabilityError::PermissionDenied(
                "Network capability only permits network access".into(),
            ))),
        }
    }
}

#[test]
fn test_capability_integration() {
    // Create file capabilities
    let read_cap = FileCapability::read_only("/tmp");
    let write_cap = FileCapability::write_only("/tmp");
    let read_write_cap = FileCapability::read_write("/tmp");

    // Test file access requests
    let read_request = AccessRequest::file_read("/tmp/test.txt");
    let write_request = AccessRequest::file_write("/tmp/test.txt");

    // Check permissions
    assert!(read_cap.permits(&read_request).is_ok());
    assert!(read_cap.permits(&write_request).is_err());

    assert!(write_cap.permits(&read_request).is_err());
    assert!(write_cap.permits(&write_request).is_ok());

    assert!(read_write_cap.permits(&read_request).is_ok());
    assert!(read_write_cap.permits(&write_request).is_ok());

    // Test path restrictions
    let outside_request = AccessRequest::file_read("/etc/passwd");
    assert!(read_cap.permits(&outside_request).is_err());

    // Test the check_capability macro
    let result = (|| -> Result<()> {
        check_capability!(read_cap, &read_request);
        Ok(())
    })();
    assert!(result.is_ok());

    let result = (|| -> Result<()> {
        check_capability!(read_cap, &write_request);
        Ok(())
    })();
    assert!(result.is_err());

    // Test the with_capability macro
    let result: Result<i32> = with_capability!(read_cap, &read_request, { Ok(42) });
    assert_eq!(result.unwrap(), 42);

    let result: Result<i32> = with_capability!(read_cap, &write_request, { Ok(42) });
    assert!(result.is_err());

    // Test network capabilities
    let outbound_cap = NetworkCapability::outbound("example.com", (80, 443));
    let inbound_cap = NetworkCapability::inbound("0.0.0.0", (8000, 9000));

    let outbound_request = AccessRequest::network_connect("example.com", 443);
    let inbound_request = AccessRequest::network_listen("0.0.0.0", 8080);

    assert!(outbound_cap.permits(&outbound_request).is_ok());
    assert!(outbound_cap.permits(&inbound_request).is_err());

    assert!(inbound_cap.permits(&outbound_request).is_err());
    assert!(inbound_cap.permits(&inbound_request).is_ok());

    // Test capability constraints
    let tmp_cap = FileCapability::read_write("/tmp");
    let constraints = [
        Constraint::FilePath("/tmp/specific".into()),
        Constraint::FileOperation {
            read: true,
            write: false,
            execute: false,
        },
    ];

    let constrained_cap = tmp_cap.constrain(&constraints).unwrap();

    // Should allow read access to /tmp/specific/file.txt
    assert!(constrained_cap
        .permits(&AccessRequest::file_read("/tmp/specific/file.txt"))
        .is_ok());

    // Should deny write access to /tmp/specific/file.txt
    assert!(constrained_cap
        .permits(&AccessRequest::file_write("/tmp/specific/file.txt"))
        .is_err());

    // Should deny access to /tmp/other/file.txt
    assert!(constrained_cap
        .permits(&AccessRequest::file_read("/tmp/other/file.txt"))
        .is_err());
}

#[test]
fn test_workflow_integration() {
    // Create a workflow
    let mut workflow = Workflow::new("Data Processing", "Process and analyze data");

    // Add nodes to the workflow
    let node1 = WorkflowNode::new_plugin_call("Load Data", "data_loader", "load_csv");
    let node1_id = node1.id;
    workflow.add_node(node1);

    let node2 = WorkflowNode::new_plugin_call("Process Data", "data_processor", "process");
    let node2_id = node2.id;
    workflow.add_node(node2);

    let mut node3 =
        WorkflowNode::new_plugin_call("Visualize Results", "visualizer", "create_chart");
    let node3_id = node3.id;
    node3.add_dependency(node2_id);
    workflow.add_node(node3.clone());

    // Validate the workflow
    assert!(workflow.validate().is_ok());

    // Test entry nodes
    let entry_nodes = workflow.entry_nodes();
    assert_eq!(entry_nodes.len(), 2);
    assert!(entry_nodes.iter().any(|n| n.id == node1_id));
    assert!(entry_nodes.iter().any(|n| n.id == node2_id));

    // Test node dependencies
    let dependents = workflow.get_dependents(&node2_id);
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0].id, node3_id);

    // Add a cyclic dependency to create an invalid workflow
    let mut invalid_workflow = workflow.clone();
    let node3_id = node3.id;

    // Make node2 depend on node3, creating a cycle: node2 -> node3 -> node2
    let mut node2 = invalid_workflow.get_node_mut(&node2_id).unwrap();
    node2.add_dependency(node3_id);

    // Validate should fail due to the cycle
    assert!(invalid_workflow.validate().is_err());

    // Test execution options
    let options = ExecutionOptions {
        input: Some(serde_json::json!({"file_path": "/path/to/data.csv"})),
        timeout_ms: Some(60000),
        max_concurrency: Some(10),
        enable_checkpointing: true,
        tags: {
            let mut map = HashMap::new();
            map.insert("environment".to_string(), "test".to_string());
            map
        },
        callback_url: Some("https://example.com/callback".to_string()),
    };

    assert_eq!(options.timeout_ms, Some(60000));
    assert_eq!(options.max_concurrency, Some(10));
    assert!(options.enable_checkpointing);
    assert_eq!(options.tags.get("environment").unwrap(), "test");
}

#[test]
fn test_id_and_config_integration() {
    // Test ID creation and parsing
    let plugin_id = PluginId::new();
    let id_str = plugin_id.to_string();
    let parsed_id = PluginId::from_str(&id_str).unwrap();
    assert_eq!(plugin_id, parsed_id);

    // Create IDs of different types with same UUID
    let uuid = plugin_id.uuid();
    let capability_id = CapabilityId::from_uuid(uuid);

    // They should have the same string representation but be different types
    assert_eq!(plugin_id.to_string(), capability_id.to_string());

    // Test configuration
    let mut config = Config::new_map();

    // Set various types of values
    config.set("app.name", "Lion Test").unwrap();
    config.set("app.version", "1.0.0").unwrap();
    config.set("app.timeout_ms", 5000).unwrap();
    config.set("app.debug", true).unwrap();
    config.set("app.tags[0]", "test").unwrap();
    config.set("app.tags[1]", "integration").unwrap();

    // Get values with specific types
    assert_eq!(config.get_as::<String>("app.name").unwrap(), "Lion Test");
    assert_eq!(config.get_as::<String>("app.version").unwrap(), "1.0.0");
    assert_eq!(config.get_as::<i64>("app.timeout_ms").unwrap(), 5000);
    assert_eq!(config.get_as::<bool>("app.debug").unwrap(), true);

    // Test array access
    let tags = config.get("app.tags").unwrap();
    assert!(tags.is_array());
    assert_eq!(tags.get_index(0).unwrap().as_str().unwrap(), "test");
    assert_eq!(tags.get_index(1).unwrap().as_str().unwrap(), "integration");

    // Test path manipulation
    config.set("nested.path.to.value", 42).unwrap();
    assert_eq!(config.get_as::<i64>("nested.path.to.value").unwrap(), 42);

    // Test path removal
    assert!(config.contains("app.name"));
    config.remove("app.name");
    assert!(!config.contains("app.name"));
}

#[test]
fn test_plugin_and_memory_integration() {
    // Test plugin configuration
    let config = PluginConfig {
        max_memory_bytes: Some(100 * 1024 * 1024), // 100 MB
        max_cpu_time_us: Some(10 * 1000 * 1000),   // 10 seconds
        function_timeout_ms: Some(5000),           // 5 seconds
        max_instances: Some(10),
        min_instances: Some(1),
        enable_hot_reload: Some(true),
        enable_debug: Some(false),
        options: serde_json::json!({"log_level": "info"}),
    };

    assert_eq!(config.max_memory_bytes, Some(100 * 1024 * 1024));
    assert_eq!(config.max_cpu_time_us, Some(10 * 1000 * 1000));
    assert_eq!(config.function_timeout_ms, Some(5000));

    // Test plugin metadata
    let plugin_id = PluginId::new();
    let metadata = PluginMetadata {
        id: plugin_id,
        name: "Test Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "A test plugin for integration testing".to_string(),
        plugin_type: lion_core::types::PluginType::Wasm,
        state: PluginState::Ready,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        functions: vec!["test_function".to_string(), "another_function".to_string()],
    };

    assert_eq!(metadata.id, plugin_id);
    assert_eq!(metadata.name, "Test Plugin");
    assert_eq!(metadata.state, PluginState::Ready);
    assert_eq!(metadata.functions.len(), 2);

    // Test plugin state transitions
    assert!(PluginState::Created.can_transition_to(PluginState::Ready));
    assert!(PluginState::Ready.can_transition_to(PluginState::Running));
    assert!(!PluginState::Created.can_transition_to(PluginState::Running));
    assert!(!PluginState::Terminated.can_transition_to(PluginState::Ready));

    // Test memory region
    let region = MemoryRegion::new(1024, MemoryRegionType::Shared);
    assert_eq!(region.size, 1024);
    assert_eq!(region.region_type, MemoryRegionType::Shared);
    assert!(region.is_shareable());
    assert!(region.allows_write());

    let read_only_region = MemoryRegion::new_read_only(1024);
    assert_eq!(read_only_region.region_type, MemoryRegionType::ReadOnly);
    assert!(read_only_region.is_shareable());
    assert!(!read_only_region.allows_write());
}

#[test]
fn test_utils_integration() {
    // Test logging
    log_event!(LogLevel::Info, "Test message");
    log_event!(LogLevel::Warning, "Test warning", 
              code => 42,
              module => "integration_test");

    assert!(LogLevel::Error > LogLevel::Warning);
    assert!(LogLevel::Warning > LogLevel::Info);
    assert!(LogLevel::Info > LogLevel::Debug);
    assert!(LogLevel::Debug > LogLevel::Trace);

    // Test version utils
    let version = Version::from_str("1.2.3").unwrap();
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 2);
    assert_eq!(version.patch, 3);

    let version_with_pre = Version::from_str("1.2.3-alpha.1").unwrap();
    assert_eq!(version_with_pre.prerelease, Some("alpha.1".to_string()));

    assert!(version_with_pre < version);
    assert!(version.is_compatible_with(&Version::from_str("1.2.4").unwrap()));
    assert!(version.is_compatible_with(&Version::from_str("1.3.0").unwrap()));
    assert!(!version.is_compatible_with(&Version::from_str("2.0.0").unwrap()));

    // Test config utils
    let mut config_value = ConfigValue::Map(HashMap::new());
    config_value.set("name", "Lion");
    config_value.set("version", "1.0.0");
    config_value.set("debug", true);

    assert_eq!(config_value.get("name").unwrap().as_str(), Some("Lion"));
    assert_eq!(config_value.get("version").unwrap().as_str(), Some("1.0.0"));
    assert_eq!(config_value.get("debug").unwrap().as_bool(), Some(true));
}

#[test]
fn test_error_hierarchy() {
    // Test error conversion
    let capability_error = CapabilityError::PermissionDenied("Access denied".to_string());
    let error: Error = capability_error.into();

    match &error {
        Error::Capability(e) => match e {
            CapabilityError::PermissionDenied(msg) => assert_eq!(msg, "Access denied"),
            _ => panic!("Expected PermissionDenied error"),
        },
        _ => panic!("Expected Capability error"),
    }

    // Test error display
    let error_str = error.to_string();
    assert!(error_str.contains("Permission denied"));
    assert!(error_str.contains("Access denied"));
}

// This test demonstrates how all the components can work together in a simple scenario
#[test]
fn test_complete_scenario() {
    // 1. Set up plugin configuration
    let mut config = Config::new_map();
    config
        .set("plugins.data_processor.max_memory_mb", 100)
        .unwrap();
    config
        .set("plugins.data_processor.timeout_ms", 5000)
        .unwrap();

    let plugin_config = PluginConfig {
        max_memory_bytes: Some(
            config
                .get_as::<i64>("plugins.data_processor.max_memory_mb")
                .unwrap() as usize
                * 1024
                * 1024,
        ),
        function_timeout_ms: config
            .get_as::<i64>("plugins.data_processor.timeout_ms")
            .map(|v| v as u64),
        ..PluginConfig::default()
    };

    // 2. Create plugin metadata
    let plugin_id = PluginId::new();
    let metadata = PluginMetadata {
        id: plugin_id,
        name: "Data Processor".to_string(),
        version: "1.0.0".to_string(),
        description: "Processes data files".to_string(),
        plugin_type: lion_core::types::PluginType::Wasm,
        state: PluginState::Ready,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        functions: vec!["process_csv".to_string(), "process_json".to_string()],
    };

    // 3. Create capabilities for the plugin
    let file_cap = FileCapability::read_write("/data");
    let network_cap = NetworkCapability::outbound("api.example.com", (80, 443));

    // 4. Define a workflow that uses the plugin
    let mut workflow = Workflow::new("Data Analysis", "Analyze CSV data");

    // Add a node to load data
    let load_node = WorkflowNode::new_plugin_call("Load CSV", "data_loader", "load_csv");
    let load_node_id = load_node.id;
    workflow.add_node(load_node);

    // Add a node to process data
    let mut process_node =
        WorkflowNode::new_plugin_call("Process Data", plugin_id.to_string(), "process_csv");
    process_node.add_dependency(load_node_id);
    process_node.set_error_policy(ErrorPolicy::Retry { max_attempts: 3 });
    let process_node_id = process_node.id;
    workflow.add_node(process_node);

    // Add a node to visualize results
    let mut visualize_node =
        WorkflowNode::new_plugin_call("Visualize", "visualizer", "create_chart");
    visualize_node.add_dependency(process_node_id);
    workflow.add_node(visualize_node);

    // 5. Validate the workflow
    assert!(workflow.validate().is_ok());

    // 6. Create execution options for the workflow
    let execution_options = ExecutionOptions {
        input: Some(serde_json::json!({
            "file_path": "/data/sales.csv",
            "columns": ["date", "sales", "region"],
        })),
        timeout_ms: Some(60000),
        max_concurrency: Some(2),
        enable_checkpointing: true,
        tags: {
            let mut map = HashMap::new();
            map.insert("environment".to_string(), "test".to_string());
            map.insert("priority".to_string(), "high".to_string());
            map
        },
        callback_url: Some("https://example.com/callback".to_string()),
    };

    // 7. Simulate a capability check for accessing the input file
    let file_request = AccessRequest::file_read("/data/sales.csv");
    assert!(file_cap.permits(&file_request).is_ok());

    // 8. Simulate a capability check for API access
    let network_request = AccessRequest::network_connect("api.example.com", 443);
    assert!(network_cap.permits(&network_request).is_ok());

    // 9. Log the workflow execution
    log_event!(LogLevel::Info, "Starting workflow execution", 
              workflow_id => workflow.id.to_string(),
              node_count => workflow.nodes.len());

    // This test demonstrates how all the components (plugin configuration, metadata,
    // capabilities, workflows, error handling, logging) can work together in a
    // complete scenario.
}
