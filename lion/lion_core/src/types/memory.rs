//! Memory-related data types.
//! 
//! This module defines data structures for memory regions and memory access.

use serde::{Serialize, Deserialize};
use crate::id::{RegionId, PluginId};

/// Type of memory region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRegionType {
    /// Read-only memory region.
    ReadOnly,
    
    /// Read-write memory region.
    ReadWrite,
    
    /// Shared memory region.
    Shared,
}

/// A memory region.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// Unique region ID.
    pub id: RegionId,
    
    /// The plugin that owns the region.
    pub owner: PluginId,
    
    /// The type of the region.
    pub region_type: MemoryRegionType,
    
    /// The size of the region in bytes.
    pub size: usize,
    
    /// Whether the region is active.
    pub active: bool,
    
    /// Additional metadata.
    pub metadata: serde_json::Value,
}

impl MemoryRegion {
    /// Create a new memory region.
    pub fn new(
        id: RegionId,
        owner: PluginId,
        region_type: MemoryRegionType,
        size: usize,
    ) -> Self {
        Self {
            id,
            owner,
            region_type,
            size,
            active: true,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
    
    /// Check if a read operation is allowed.
    pub fn can_read(&self) -> bool {
        self.active
    }
    
    /// Check if a write operation is allowed.
    pub fn can_write(&self) -> bool {
        self.active && (self.region_type == MemoryRegionType::ReadWrite || self.region_type == MemoryRegionType::Shared)
    }
    
    /// Check if sharing is allowed.
    pub fn can_share(&self) -> bool {
        self.active && self.region_type == MemoryRegionType::Shared
    }
}