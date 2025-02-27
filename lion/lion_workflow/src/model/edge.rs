use crate::model::node::NodeId;
use lion_capability::model::capability::CapabilityId;
use lion_core::id::Id;
use serde::{Deserialize, Serialize};

/// Unique identifier for workflow edges
pub type EdgeId = Id<Edge>;

/// Types of conditions that can be applied to an edge
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ConditionType {
    /// No condition (always passes)
    None,
    /// JSON path condition on the source node's output
    JsonPath(String),
    /// JavaScript expression condition
    Expression(String),
    /// Custom condition handled by a plugin
    Custom {
        /// Identifier of the plugin that handles this condition
        plugin_id: String,
        /// Configuration for the condition
        config: serde_json::Value,
    },
}

/// An edge in the workflow graph, connecting two nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Edge {
    /// Unique identifier for this edge
    pub id: EdgeId,
    
    /// Source node ID
    pub source: NodeId,
    
    /// Target node ID
    pub target: NodeId,
    
    /// Optional condition that must be satisfied for this edge to be traversed
    pub condition: ConditionType,
    
    /// Capability required to traverse this edge (cross-component boundaries)
    pub required_capability: Option<CapabilityId>,
    
    /// Custom metadata for this edge
    pub metadata: serde_json::Value,
}

impl Edge {
    /// Create a new edge connecting source to target
    pub fn new(id: EdgeId, source: NodeId, target: NodeId) -> Self {
        Edge {
            id,
            source,
            target,
            condition: ConditionType::None,
            required_capability: None,
            metadata: serde_json::Value::Null,
        }
    }
    
    /// Add a condition to this edge
    pub fn with_condition(mut self, condition: ConditionType) -> Self {
        self.condition = condition;
        self
    }
    
    /// Add a JSON path condition to this edge
    pub fn with_json_path(mut self, path: &str) -> Self {
        self.condition = ConditionType::JsonPath(path.to_string());
        self
    }
    
    /// Add an expression condition to this edge
    pub fn with_expression(mut self, expr: &str) -> Self {
        self.condition = ConditionType::Expression(expr.to_string());
        self
    }
    
    /// Add a custom condition to this edge
    pub fn with_custom_condition(mut self, plugin_id: &str, config: serde_json::Value) -> Self {
        self.condition = ConditionType::Custom {
            plugin_id: plugin_id.to_string(),
            config,
        };
        self
    }
    
    /// Set the required capability for traversing this edge
    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.required_capability = Some(capability_id);
        self
    }
    
    /// Add metadata to this edge
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
    
    /// Check if this edge requires a capability
    pub fn has_capability_requirement(&self) -> bool {
        self.required_capability.is_some()
    }
    
    /// Check if this edge has a condition
    pub fn has_condition(&self) -> bool {
        self.condition != ConditionType::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_edge_creation() {
        let source = NodeId::new();
        let target = NodeId::new();
        let edge = Edge::new(EdgeId::new(), source.clone(), target.clone());
        
        assert_eq!(edge.source, source);
        assert_eq!(edge.target, target);
        assert_eq!(edge.condition, ConditionType::None);
        assert_eq!(edge.required_capability, None);
    }
    
    #[test]
    fn test_edge_with_json_path_condition() {
        let source = NodeId::new();
        let target = NodeId::new();
        let edge = Edge::new(EdgeId::new(), source.clone(), target.clone())
            .with_json_path("$.result.success");
        
        assert_eq!(edge.condition, ConditionType::JsonPath("$.result.success".to_string()));
        assert!(edge.has_condition());
    }
    
    #[test]
    fn test_edge_with_capability() {
        let source = NodeId::new();
        let target = NodeId::new();
        let capability = CapabilityId::new();
        let edge = Edge::new(EdgeId::new(), source.clone(), target.clone())
            .with_capability(capability);
        
        assert_eq!(edge.required_capability, Some(capability));
        assert!(edge.has_capability_requirement());
    }
}
