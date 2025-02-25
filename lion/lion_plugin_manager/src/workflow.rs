//! Plugin workflow management.

use crate::error::PluginManagerError;
use lion_core::message::MessageBus;
use lion_core::plugin::PluginId;
use std::sync::Arc;

/// A linear chain of plugins for sequential processing
pub struct PluginChain {
    /// The steps in the chain
    steps: Vec<PluginId>,
}

impl PluginChain {
    /// Create a new plugin chain
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }
    
    /// Create a new plugin chain with the given steps
    pub fn with_steps(steps: Vec<PluginId>) -> Self {
        Self { steps }
    }
    
    /// Add a step to the chain
    pub fn add_step(&mut self, plugin_id: PluginId) {
        self.steps.push(plugin_id);
    }
    
    /// Get the steps in the chain
    pub fn steps(&self) -> &[PluginId] {
        &self.steps
    }
    
    /// Start the chain with the given message
    pub fn start(
        &self,
        message_bus: &Arc<dyn MessageBus>,
        initial_sender: PluginId,
        message: serde_json::Value,
    ) -> Result<(), PluginManagerError> {
        if let Some(first) = self.steps.first() {
            message_bus
                .send_direct(initial_sender, *first, message.clone())
                .map_err(|e| PluginManagerError::MessagingError(format!(
                    "Failed to send message to first plugin in chain: {}",
                    e
                )))?;
            
            Ok(())
        } else {
            Err(PluginManagerError::Internal("Chain is empty".to_string()))
        }
    }
    
    /// Process the next step in the chain
    pub fn next_step(
        &self,
        message_bus: &Arc<dyn MessageBus>,
        current_plugin_id: PluginId,
        message: serde_json::Value,
    ) -> Result<bool, PluginManagerError> {
        // Find the current plugin in the chain
        let position = self.steps.iter().position(|id| *id == current_plugin_id);
        
        if let Some(position) = position {
            // Check if there's a next step
            if position + 1 < self.steps.len() {
                let next_plugin_id = self.steps[position + 1];
                
                // Send the message to the next plugin
                message_bus
                    .send_direct(current_plugin_id, next_plugin_id, message.clone())
                    .map_err(|e| PluginManagerError::MessagingError(format!(
                        "Failed to send message to next plugin in chain: {}",
                        e
                    )))?;
                
                Ok(true)
            } else {
                // This is the last step
                Ok(false)
            }
        } else {
            Err(PluginManagerError::Internal(format!(
                "Plugin {} is not in the chain",
                current_plugin_id.0
            )))
        }
    }
}

impl Default for PluginChain {
    fn default() -> Self {
        Self::new()
    }
}
