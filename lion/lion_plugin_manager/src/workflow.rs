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
            Err(PluginManagerError::WorkflowError("Chain is empty".to_string()))
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
            Err(PluginManagerError::WorkflowError(format!(
                "Plugin {} is not in the chain",
                current_plugin_id.0
            )))
        }
    }
    
    /// Execute the entire chain synchronously
    pub fn execute(
        &self,
        plugin_manager: &crate::manager::PluginManager,
        input: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, PluginManagerError> {
        if self.steps.is_empty() {
            return Err(PluginManagerError::WorkflowError("Chain is empty".to_string()));
        }
        
        let mut message = input;
        
        // Process each step in the chain
        for (i, &plugin_id) in self.steps.iter().enumerate() {
            // Get the plugin
            let plugin = plugin_manager.get_plugin(plugin_id)
                .ok_or_else(|| PluginManagerError::PluginNotFound(plugin_id.0.to_string()))?;
            
            // Handle the message
            let mut plugin_clone = plugin.clone();
            
            // Initialize if needed
            if plugin_clone.state() == lion_core::plugin::PluginState::Created {
                plugin_clone.initialize()
                    .map_err(|e| PluginManagerError::InitializationFailure(e.to_string()))?;
            }
            
            // Process the message
            let result = plugin_clone.handle_message(message)
                .map_err(|e| PluginManagerError::WorkflowError(format!(
                    "Failed to process message in plugin {}: {}",
                    plugin_id.0, e
                )))?;
            
            // Use the result as the input for the next plugin
            if let Some(result_value) = result {
                message = result_value;
            } else if i < self.steps.len() - 1 {
                // If a plugin returns None but it's not the last one, that's an error
                return Err(PluginManagerError::WorkflowError(format!(
                    "Plugin {} returned no result",
                    plugin_id.0
                )));
            }
        }
        
        Ok(Some(message))
    }
}

impl Default for PluginChain {
    fn default() -> Self {
        Self::new()
    }
}