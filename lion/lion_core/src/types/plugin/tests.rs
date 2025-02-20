use super::*;
use serde_json::json;

struct MockPlugin {
    manifest: PluginManifest,
    state: PluginState,
}

#[async_trait]
impl Plugin for MockPlugin {
    async fn initialize(&mut self, _config: Value) -> Result<(), String> {
        self.state = PluginState::Ready;
        Ok(())
    }

    async fn execute(&self, input: Value) -> Result<Value, String> {
        Ok(json!({
            "input": input,
            "processed": true
        }))
    }

    async fn cleanup(&mut self) -> Result<(), String> {
        self.state = PluginState::Disabled;
        Ok(())
    }

    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn state(&self) -> PluginState {
        self.state
    }
}

impl Identifiable for MockPlugin {
    fn id(&self) -> Uuid {
        self.manifest.id
    }
}

impl Describable for MockPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn description(&self) -> &str {
        &self.manifest.description
    }
}

impl Versionable for MockPlugin {
    fn version(&self) -> String {
        self.manifest.version.clone()
    }

    fn is_compatible_with(&self, other_version: &str) -> bool {
        // Simple version check for testing
        self.version() == other_version
    }
}

impl Validatable for MockPlugin {
    type Error = String;

    fn validate(&self) -> Result<(), Self::Error> {
        self.manifest.validate()
    }
}

impl MockPlugin {
    fn new(name: &str, version: &str, description: &str) -> Self {
        Self {
            manifest: PluginManifest::new(
                name.to_string(),
                version.to_string(),
                description.to_string(),
            ),
            state: PluginState::Uninitialized,
        }
    }
}

#[tokio::test]
async fn test_plugin_lifecycle() {
    let mut plugin = MockPlugin::new("test", "1.0.0", "A test plugin");
    
    // Test initial state
    assert_eq!(plugin.state(), PluginState::Uninitialized);
    
    // Test initialization
    plugin.initialize(json!({})).await.unwrap();
    assert_eq!(plugin.state(), PluginState::Ready);
    
    // Test execution
    let result = plugin.execute(json!({"test": true})).await.unwrap();
    assert!(result["processed"].as_bool().unwrap());
    
    // Test cleanup
    plugin.cleanup().await.unwrap();
    assert_eq!(plugin.state(), PluginState::Disabled);
}

#[test]
fn test_plugin_manifest() {
    let manifest = PluginManifest::new(
        "test-plugin".to_string(),
        "1.0.0".to_string(),
        "A test plugin".to_string(),
    );

    assert!(!manifest.id.is_nil());
    assert_eq!(manifest.name, "test-plugin");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.description, "A test plugin");
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.capabilities.is_empty());
}

#[test]
fn test_plugin_manifest_validation() {
    // Valid manifest
    let manifest = PluginManifest::new(
        "test".to_string(),
        "1.0.0".to_string(),
        "description".to_string(),
    );
    assert!(manifest.validate().is_ok());

    // Invalid manifest - empty name
    let manifest = PluginManifest::new(
        "".to_string(),
        "1.0.0".to_string(),
        "description".to_string(),
    );
    assert!(manifest.validate().is_err());

    // Invalid manifest - empty version
    let manifest = PluginManifest::new(
        "test".to_string(),
        "".to_string(),
        "description".to_string(),
    );
    assert!(manifest.validate().is_err());

    // Invalid manifest - empty description
    let manifest = PluginManifest::new(
        "test".to_string(),
        "1.0.0".to_string(),
        "".to_string(),
    );
    assert!(manifest.validate().is_err());
}

#[test]
fn test_wasm_path_resolution() {
    let mut manifest = PluginManifest::new(
        "test-plugin".to_string(),
        "1.0.0".to_string(),
        "A test plugin".to_string(),
    );
    manifest.wasm_path = Some("plugin.wasm".to_string());

    let manifest_dir = PathBuf::from("/plugins/test-plugin");
    let wasm_path = manifest.resolve_wasm_path(&manifest_dir).unwrap();
    assert_eq!(wasm_path, PathBuf::from("/plugins/test-plugin/plugin.wasm"));
}