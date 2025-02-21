use crate::plugin_manager::error::PluginError;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct PluginsConfig {
    pub data_dir: PathBuf,
    pub calculator_manifest: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub plugins: PluginsConfig,
}

impl Config {
    pub fn load<P: AsRef<Path>>(config_path: P) -> Result<Self, PluginError> {
        let content = fs::read_to_string(config_path)
            .map_err(|e| PluginError::ConfigError(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| PluginError::ConfigError(format!("Failed to parse config file: {}", e)))
    }

    pub fn from_project_root() -> Result<Self, PluginError> {
        // Try to find Lion.toml in current directory or parent directories
        let mut current_dir = std::env::current_dir().map_err(|e| {
            PluginError::ConfigError(format!("Failed to get current directory: {}", e))
        })?;

        loop {
            let config_path = current_dir.join("Lion.toml");
            if config_path.exists() {
                return Self::load(config_path);
            }

            if !current_dir.pop() {
                break;
            }
        }

        Err(PluginError::ConfigError(
            "Could not find Lion.toml".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_load_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("Lion.toml");

        let config_content = r#"
[plugins]
data_dir = "plugins/data"
calculator_manifest = "plugins/calculator/manifest.toml"
"#;
        fs::write(&config_path, config_content).unwrap();

        let config = Config::load(config_path).unwrap();
        assert_eq!(config.plugins.data_dir, PathBuf::from("plugins/data"));
        assert_eq!(
            config.plugins.calculator_manifest,
            PathBuf::from("plugins/calculator/manifest.toml")
        );
    }

    #[test]
    fn test_invalid_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("Lion.toml");

        let invalid_content = r#"
[plugins]
invalid = true
"#;
        fs::write(&config_path, invalid_content).unwrap();

        let result = Config::load(config_path);
        assert!(result.is_err());
    }
}
