mod config;
mod core;
mod discovery;
mod error;
mod loader;
mod manifest;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;

pub use config::{Config, PluginsConfig};
pub use core::PluginManager;
pub use error::PluginError;
pub use manifest::{PluginFunction, PluginManifest};

// Re-export test utilities for other modules to use
#[cfg(test)]
pub use test_utils::init_test_logging;
