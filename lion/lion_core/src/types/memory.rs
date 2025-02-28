//! Memory management types.
//!
//! This module defines data structures for memory management in the Lion system,
//! including memory regions and memory region types. These are used in the
//! isolation system to control memory access between plugins and the host.
//!
//! The memory model is based on research in "Zero-Copy Data Transfer in Wasmtime
//! for Lion Plugin System" research.

use crate::id::RegionId;
use serde::{Deserialize, Serialize};

/// Type of memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRegionType {
    /// Private memory region, accessible only to the creator.
    Private,

    /// Shared memory region, can be shared with other plugins.
    Shared,

    /// Read-only memory region, can be shared with read-only access.
    ReadOnly,

    /// Ring buffer memory region, used for efficient message passing.
    RingBuffer,

    /// Direct memory region, maps to a host memory address.
    Direct,
}

impl MemoryRegionType {
    /// Checks if this memory region type can be shared with other plugins.
    ///
    /// # Returns
    ///
    /// `true` if the region can be shared, `false` otherwise.
    pub fn is_shareable(&self) -> bool {
        match self {
            Self::Private => false,
            Self::Shared | Self::ReadOnly | Self::RingBuffer | Self::Direct => true,
        }
    }

    /// Checks if this memory region type allows write access.
    ///
    /// # Returns
    ///
    /// `true` if the region allows write access, `false` otherwise.
    pub fn allows_write(&self) -> bool {
        match self {
            Self::Private | Self::Shared | Self::RingBuffer | Self::Direct => true,
            Self::ReadOnly => false,
        }
    }

    /// Get a human-readable name for this memory region type.
    ///
    /// # Returns
    ///
    /// A string representation of this memory region type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "Private",
            Self::Shared => "Shared",
            Self::ReadOnly => "ReadOnly",
            Self::RingBuffer => "RingBuffer",
            Self::Direct => "Direct",
        }
    }
}

/// A memory region in the system.
///
/// Memory regions are used to control memory access between plugins and the host.
/// They can be shared between plugins, allowing for efficient data transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// Unique identifier for this memory region.
    pub id: RegionId,

    /// Size of the memory region in bytes.
    pub size: usize,

    /// Type of memory region.
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    /// Create a new memory region.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the memory region in bytes.
    /// * `region_type` - Type of memory region.
    ///
    /// # Returns
    ///
    /// A new memory region with a unique ID.
    pub fn new(size: usize, region_type: MemoryRegionType) -> Self {
        Self {
            id: RegionId::new(),
            size,
            region_type,
        }
    }

    /// Create a new private memory region.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new private memory region with a unique ID.
    pub fn new_private(size: usize) -> Self {
        Self::new(size, MemoryRegionType::Private)
    }

    /// Create a new shared memory region.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new shared memory region with a unique ID.
    pub fn new_shared(size: usize) -> Self {
        Self::new(size, MemoryRegionType::Shared)
    }

    /// Create a new read-only memory region.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new read-only memory region with a unique ID.
    pub fn new_read_only(size: usize) -> Self {
        Self::new(size, MemoryRegionType::ReadOnly)
    }

    /// Create a new ring buffer memory region.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new ring buffer memory region with a unique ID.
    pub fn new_ring_buffer(size: usize) -> Self {
        Self::new(size, MemoryRegionType::RingBuffer)
    }

    /// Check if this memory region can be shared with other plugins.
    ///
    /// # Returns
    ///
    /// `true` if the region can be shared, `false` otherwise.
    pub fn is_shareable(&self) -> bool {
        self.region_type.is_shareable()
    }

    /// Check if this memory region allows write access.
    ///
    /// # Returns
    ///
    /// `true` if the region allows write access, `false` otherwise.
    pub fn allows_write(&self) -> bool {
        self.region_type.allows_write()
    }
}

/// Memory access permission.
///
/// This enum represents the different types of access that can be
/// granted to a memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryAccess {
    /// Read-only access.
    ReadOnly,

    /// Read-write access.
    ReadWrite,
}

impl MemoryAccess {
    /// Check if this access permission allows read operations.
    ///
    /// # Returns
    ///
    /// `true` if read operations are allowed, `false` otherwise.
    pub fn can_read(&self) -> bool {
        match self {
            Self::ReadOnly | Self::ReadWrite => true,
        }
    }

    /// Check if this access permission allows write operations.
    ///
    /// # Returns
    ///
    /// `true` if write operations are allowed, `false` otherwise.
    pub fn can_write(&self) -> bool {
        match self {
            Self::ReadOnly => false,
            Self::ReadWrite => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_type() {
        // Test is_shareable
        assert!(!MemoryRegionType::Private.is_shareable());
        assert!(MemoryRegionType::Shared.is_shareable());
        assert!(MemoryRegionType::ReadOnly.is_shareable());
        assert!(MemoryRegionType::RingBuffer.is_shareable());
        assert!(MemoryRegionType::Direct.is_shareable());

        // Test allows_write
        assert!(MemoryRegionType::Private.allows_write());
        assert!(MemoryRegionType::Shared.allows_write());
        assert!(!MemoryRegionType::ReadOnly.allows_write());
        assert!(MemoryRegionType::RingBuffer.allows_write());
        assert!(MemoryRegionType::Direct.allows_write());

        // Test as_str
        assert_eq!(MemoryRegionType::Private.as_str(), "Private");
        assert_eq!(MemoryRegionType::Shared.as_str(), "Shared");
        assert_eq!(MemoryRegionType::ReadOnly.as_str(), "ReadOnly");
        assert_eq!(MemoryRegionType::RingBuffer.as_str(), "RingBuffer");
        assert_eq!(MemoryRegionType::Direct.as_str(), "Direct");
    }

    #[test]
    fn test_memory_region() {
        // Test new
        let region = MemoryRegion::new(1024, MemoryRegionType::Shared);
        assert_eq!(region.size, 1024);
        assert_eq!(region.region_type, MemoryRegionType::Shared);

        // Test factory methods
        let private = MemoryRegion::new_private(1024);
        assert_eq!(private.region_type, MemoryRegionType::Private);

        let shared = MemoryRegion::new_shared(1024);
        assert_eq!(shared.region_type, MemoryRegionType::Shared);

        let read_only = MemoryRegion::new_read_only(1024);
        assert_eq!(read_only.region_type, MemoryRegionType::ReadOnly);

        let ring_buffer = MemoryRegion::new_ring_buffer(1024);
        assert_eq!(ring_buffer.region_type, MemoryRegionType::RingBuffer);

        // Test is_shareable
        assert!(!private.is_shareable());
        assert!(shared.is_shareable());

        // Test allows_write
        assert!(private.allows_write());
        assert!(shared.allows_write());
        assert!(!read_only.allows_write());
    }

    #[test]
    fn test_memory_access() {
        // Test can_read
        assert!(MemoryAccess::ReadOnly.can_read());
        assert!(MemoryAccess::ReadWrite.can_read());

        // Test can_write
        assert!(!MemoryAccess::ReadOnly.can_write());
        assert!(MemoryAccess::ReadWrite.can_write());
    }

    #[test]
    fn test_serialization() {
        // Test MemoryRegionType serialization
        let region_type = MemoryRegionType::Shared;
        let serialized = serde_json::to_string(&region_type).unwrap();
        let deserialized: MemoryRegionType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(region_type, deserialized);

        // Test MemoryRegion serialization
        let region = MemoryRegion::new_shared(1024);
        let serialized = serde_json::to_string(&region).unwrap();
        let deserialized: MemoryRegion = serde_json::from_str(&serialized).unwrap();
        assert_eq!(region.size, deserialized.size);
        assert_eq!(region.region_type, deserialized.region_type);
        assert_eq!(region.id, deserialized.id);

        // Test MemoryAccess serialization
        let access = MemoryAccess::ReadWrite;
        let serialized = serde_json::to_string(&access).unwrap();
        let deserialized: MemoryAccess = serde_json::from_str(&serialized).unwrap();
        assert_eq!(access, deserialized);
    }
}
