use bitflags::bitflags;
use std::any::Any;
use std::cmp::{max, min};
use std::collections::BTreeMap;

use super::capability::{AccessRequest, Capability, CapabilityError, Constraint};

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    /// Represents memory operation permissions as a bit field
    pub struct MemoryOperations: u8 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
    }
}

/// A range of memory addresses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRange {
    /// Base address of the range
    pub base: usize,
    /// Size of the range in bytes
    pub size: usize,
    /// Operations permitted on this range
    pub operations: MemoryOperations,
}

impl MemoryRange {
    /// Creates a new memory range
    pub fn new(base: usize, size: usize, operations: MemoryOperations) -> Self {
        Self {
            base,
            size,
            operations,
        }
    }

    /// The end address of this range (exclusive)
    pub fn end(&self) -> usize {
        self.base + self.size
    }

    /// Returns true if this range contains the given address
    pub fn contains(&self, address: usize) -> bool {
        address >= self.base && address < self.end()
    }

    /// Returns true if this range fully contains another range
    pub fn contains_range(&self, other: &MemoryRange) -> bool {
        self.base <= other.base && self.end() >= other.end()
    }

    /// Returns true if this range overlaps with another range
    pub fn overlaps(&self, other: &MemoryRange) -> bool {
        self.base < other.end() && other.base < self.end()
    }

    /// Returns the intersection of this range with another
    pub fn intersect(&self, other: &MemoryRange) -> Option<MemoryRange> {
        if !self.overlaps(other) {
            return None;
        }

        let base = max(self.base, other.base);
        let end = min(self.end(), other.end());
        let size = end - base;

        // Combine operations (intersection)
        let operations = self.operations & other.operations;

        if operations.is_empty() {
            return None;
        }

        Some(MemoryRange::new(base, size, operations))
    }

    /// Returns true if this range has a subset of the permissions of another range
    pub fn is_permission_subset_of(&self, other: &MemoryRange) -> bool {
        (self.operations & other.operations).bits() == self.operations.bits()
    }
}

/// Represents a capability to access memory regions
#[derive(Debug, Clone)]
pub struct MemoryCapability {
    /// Map of memory ranges sorted by base address
    ranges: BTreeMap<usize, MemoryRange>,
}

impl MemoryCapability {
    /// Creates a new memory capability with the given ranges
    pub fn new(ranges: Vec<MemoryRange>) -> Self {
        let mut map = BTreeMap::new();
        for range in ranges {
            map.insert(range.base, range);
        }
        Self { ranges: map }
    }

    /// Adds a range to this capability
    pub fn add_range(&mut self, range: MemoryRange) {
        self.ranges.insert(range.base, range);
    }

    /// Gets the ranges in this capability
    pub fn ranges(&self) -> impl Iterator<Item = &MemoryRange> {
        self.ranges.values()
    }

    /// Finds the range containing the given address, if any
    fn find_range_containing(&self, address: usize) -> Option<&MemoryRange> {
        // Find the last range that starts at or before the address
        let mut candidate = None;

        // Find ranges where base <= address
        for (_, range) in self.ranges.range(..=address) {
            if range.contains(address) {
                candidate = Some(range);
            }
        }

        candidate
    }

    /// Finds all ranges that overlap with the given range
    fn find_overlapping_ranges(&self, start: usize, end: usize) -> Vec<&MemoryRange> {
        let mut result = Vec::new();

        // Check ranges where base < end
        for (_, range) in self.ranges.range(..end) {
            if range.base < end && range.end() > start {
                result.push(range);
            }
        }

        result
    }

    /// Applies a memory range constraint to this capability
    fn apply_range_constraint(
        &self,
        base: usize,
        size: usize,
        read: bool,
        write: bool,
    ) -> Result<Self, CapabilityError> {
        let request_range = MemoryRange::new(base, size, {
            let mut ops = MemoryOperations::empty();
            if read {
                ops |= MemoryOperations::READ;
            }
            if write {
                ops |= MemoryOperations::WRITE;
            }
            ops
        });

        // Find overlapping ranges
        let overlapping = self.find_overlapping_ranges(base, base + size);

        if overlapping.is_empty() {
            return Err(CapabilityError::InvalidConstraint(format!(
                "Memory range {}:{} is not covered by this capability",
                base, size
            )));
        }

        // Create a new capability with the intersections
        let mut new_ranges = Vec::new();

        for range in overlapping {
            if let Some(intersection) = range.intersect(&request_range) {
                new_ranges.push(intersection);
            }
        }

        if new_ranges.is_empty() {
            return Err(CapabilityError::InvalidConstraint(format!(
                "No valid permissions for memory range {}:{}",
                base, size
            )));
        }

        Ok(Self::new(new_ranges))
    }
}

impl Capability for MemoryCapability {
    fn capability_type(&self) -> &str {
        "memory"
    }

    fn permits(&self, request: &AccessRequest) -> Result<(), CapabilityError> {
        match request {
            AccessRequest::Memory {
                address,
                size,
                read,
                write,
            } => {
                // Quick check for zero-size requests
                if *size == 0 {
                    return Ok(());
                }

                let end_address = address + size;

                // Find all ranges that overlap with the request
                let overlapping = self.find_overlapping_ranges(*address, end_address);

                if overlapping.is_empty() {
                    return Err(CapabilityError::AccessDenied(format!(
                        "Memory range {}:{} is not covered by this capability",
                        address, size
                    )));
                }

                // Check if the ranges collectively cover the entire request range
                let mut covered_end = *address;

                // Sort by base address
                let mut sorted_ranges = overlapping;
                sorted_ranges.sort_by_key(|r| r.base);

                for range in sorted_ranges {
                    // There's a gap in coverage
                    if range.base > covered_end {
                        return Err(CapabilityError::AccessDenied(format!(
                            "Memory range {}:{} has gaps in coverage",
                            address, size
                        )));
                    }

                    // Check operations
                    let mut required_ops = MemoryOperations::empty();
                    if *read {
                        required_ops |= MemoryOperations::READ;
                    }
                    if *write {
                        required_ops |= MemoryOperations::WRITE;
                    }

                    if !range.operations.contains(required_ops) {
                        return Err(CapabilityError::AccessDenied(format!(
                            "Operation not permitted on memory range {}:{}",
                            address, size
                        )));
                    }

                    // Update covered range
                    covered_end = std::cmp::max(covered_end, range.end());

                    // If we've covered the entire request, we're done
                    if covered_end >= end_address {
                        return Ok(());
                    }
                }

                // If we get here, there's a gap at the end
                Err(CapabilityError::AccessDenied(format!(
                    "Memory range {}:{} not fully covered by this capability",
                    address, size
                )))
            }
            _ => Err(CapabilityError::IncompatibleTypes(format!(
                "Expected Memory request, got {:?}",
                request
            ))),
        }
    }

    fn constrain(
        &self,
        constraints: &[Constraint],
    ) -> Result<Box<dyn Capability>, CapabilityError> {
        let mut result = self.clone();

        for constraint in constraints {
            match constraint {
                Constraint::MemoryRange {
                    base,
                    size,
                    read,
                    write,
                } => {
                    result = result.apply_range_constraint(*base, *size, *read, *write)?;
                }
                _ => {
                    return Err(CapabilityError::InvalidConstraint(format!(
                        "Constraint {:?} not applicable to MemoryCapability",
                        constraint
                    )))
                }
            }
        }

        Ok(Box::new(result))
    }

    fn split(&self) -> Vec<Box<dyn Capability>> {
        let mut result = Vec::new();

        // Split by individual range
        for range in self.ranges.values() {
            result.push(Box::new(MemoryCapability::new(vec![*range])) as Box<dyn Capability>);
        }

        result
    }

    fn join(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to MemoryCapability
        if let Some(other_mem) = other.as_any().downcast_ref::<MemoryCapability>() {
            // Create a union of the ranges
            let mut ranges = Vec::new();

            // Add all ranges from self
            ranges.extend(self.ranges.values().cloned());

            // Add all ranges from other
            ranges.extend(other_mem.ranges.values().cloned());

            Ok(Box::new(MemoryCapability::new(ranges)))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot join MemoryCapability with a different capability type".to_string(),
            ))
        }
    }

    fn leq(&self, other: &dyn Capability) -> bool {
        // Try to downcast the other capability to MemoryCapability
        if let Some(other_mem) = other.as_any().downcast_ref::<MemoryCapability>() {
            // For each range in self, there must be a range in other that:
            // 1. Contains it completely
            // 2. Permits all operations that self permits

            for self_range in self.ranges.values() {
                let mut is_covered = false;

                for other_range in other_mem.ranges.values() {
                    if other_range.contains_range(self_range)
                        && self_range.is_permission_subset_of(other_range)
                    {
                        is_covered = true;
                        break;
                    }
                }

                if !is_covered {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    fn meet(&self, other: &dyn Capability) -> Result<Box<dyn Capability>, CapabilityError> {
        // Try to downcast the other capability to MemoryCapability
        if let Some(other_mem) = other.as_any().downcast_ref::<MemoryCapability>() {
            let mut intersections = Vec::new();

            // Find all intersections between ranges
            for self_range in self.ranges.values() {
                for other_range in other_mem.ranges.values() {
                    if let Some(intersection) = self_range.intersect(other_range) {
                        intersections.push(intersection);
                    }
                }
            }

            if intersections.is_empty() {
                return Err(CapabilityError::InvalidState(
                    "No memory ranges in common between the capabilities".to_string(),
                ));
            }

            Ok(Box::new(MemoryCapability::new(intersections)))
        } else {
            Err(CapabilityError::IncompatibleTypes(
                "Cannot compute meet with different capability types".to_string(),
            ))
        }
    }

    fn clone_box(&self) -> Box<dyn Capability> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_range_operations() {
        let range1 = MemoryRange::new(1000, 1000, MemoryOperations::READ);
        let range2 = MemoryRange::new(1500, 1000, MemoryOperations::READ | MemoryOperations::WRITE);

        // Test contains
        assert!(range1.contains(1000));
        assert!(range1.contains(1999));
        assert!(!range1.contains(2000));

        // Test overlaps
        assert!(range1.overlaps(&range2));
        assert!(range2.overlaps(&range1));

        // Test intersection
        let intersection = range1.intersect(&range2).unwrap();
        assert_eq!(intersection.base, 1500);
        assert_eq!(intersection.size, 500);
        assert_eq!(intersection.operations, MemoryOperations::READ);

        // Test non-overlapping ranges
        let range3 = MemoryRange::new(3000, 1000, MemoryOperations::READ);
        assert!(!range1.overlaps(&range3));
        assert!(range1.intersect(&range3).is_none());
    }

    #[test]
    fn test_memory_capability_permits() {
        // Create a capability with several ranges
        let ranges = vec![
            MemoryRange::new(1000, 1000, MemoryOperations::READ),
            MemoryRange::new(2000, 1000, MemoryOperations::READ | MemoryOperations::WRITE),
            MemoryRange::new(4000, 1000, MemoryOperations::WRITE),
        ];

        let cap = MemoryCapability::new(ranges);

        // Test valid read request
        assert!(cap
            .permits(&AccessRequest::Memory {
                address: 1500,
                size: 100,
                read: true,
                write: false,
            })
            .is_ok());

        // Test valid write request
        assert!(cap
            .permits(&AccessRequest::Memory {
                address: 2500,
                size: 100,
                read: false,
                write: true,
            })
            .is_ok());

        // Test invalid request (gap in coverage)
        assert!(cap
            .permits(&AccessRequest::Memory {
                address: 1500,
                size: 2000,
                read: true,
                write: false,
            })
            .is_err());

        // Test invalid request (wrong operation)
        assert!(cap
            .permits(&AccessRequest::Memory {
                address: 1500,
                size: 100,
                read: false,
                write: true,
            })
            .is_err());

        // Test invalid request (outside range)
        assert!(cap
            .permits(&AccessRequest::Memory {
                address: 8000,
                size: 100,
                read: true,
                write: false,
            })
            .is_err());
    }

    #[test]
    fn test_memory_capability_constrain() {
        // Create a capability with several ranges
        let ranges = vec![
            MemoryRange::new(1000, 1000, MemoryOperations::READ | MemoryOperations::WRITE),
            MemoryRange::new(2000, 1000, MemoryOperations::READ),
        ];

        let cap = MemoryCapability::new(ranges);

        // Constrain to specific range
        let constrained = cap
            .constrain(&[Constraint::MemoryRange {
                base: 1500,
                size: 1000,
                read: true,
                write: false,
            }])
            .unwrap();

        // The constrained capability should have two ranges:
        // 1. 1500-2000 with READ|WRITE (intersection with first range)
        // 2. 2000-2500 with READ (intersection with second range)

        assert!(constrained
            .permits(&AccessRequest::Memory {
                address: 1600,
                size: 100,
                read: true,
                write: false,
            })
            .is_ok());

        assert!(constrained
            .permits(&AccessRequest::Memory {
                address: 2100,
                size: 100,
                read: true,
                write: false,
            })
            .is_ok());

        // Should deny write to second range
        assert!(constrained
            .permits(&AccessRequest::Memory {
                address: 2100,
                size: 100,
                read: false,
                write: true,
            })
            .is_err());

        // Should deny access outside the constrained range
        assert!(constrained
            .permits(&AccessRequest::Memory {
                address: 1000,
                size: 100,
                read: true,
                write: false,
            })
            .is_err());
    }

    #[test]
    fn test_memory_capability_leq() {
        // Create two capabilities
        let ranges1 = vec![MemoryRange::new(1000, 500, MemoryOperations::READ)];

        let ranges2 = vec![MemoryRange::new(
            500,
            2000,
            MemoryOperations::READ | MemoryOperations::WRITE,
        )];

        let cap1 = MemoryCapability::new(ranges1);
        let cap2 = MemoryCapability::new(ranges2);

        // cap1 <= cap2 because cap1's range is fully contained in cap2's range
        // and cap1's operations are a subset of cap2's operations
        assert!(cap1.leq(&cap2));

        // cap2 !<= cap1 because cap2's range is not fully contained in cap1's range
        assert!(!cap2.leq(&cap1));

        // Create a capability with non-overlapping ranges
        let ranges3 = vec![MemoryRange::new(3000, 1000, MemoryOperations::READ)];

        let cap3 = MemoryCapability::new(ranges3);

        // Neither capability is <= the other
        assert!(!cap1.leq(&cap3));
        assert!(!cap3.leq(&cap1));
    }

    #[test]
    fn test_memory_capability_join_and_meet() {
        // Create two capabilities with overlapping ranges
        let ranges1 = vec![MemoryRange::new(1000, 1000, MemoryOperations::READ)];

        let ranges2 = vec![MemoryRange::new(
            1500,
            1000,
            MemoryOperations::READ | MemoryOperations::WRITE,
        )];

        let cap1 = MemoryCapability::new(ranges1);
        let cap2 = MemoryCapability::new(ranges2);

        // Join
        let join = cap1.join(&cap2).unwrap();

        // The joined capability should allow both ranges
        assert!(join
            .permits(&AccessRequest::Memory {
                address: 1200,
                size: 100,
                read: true,
                write: false,
            })
            .is_ok());

        assert!(join
            .permits(&AccessRequest::Memory {
                address: 1800,
                size: 100,
                read: true,
                write: true,
            })
            .is_ok());

        // Meet
        let meet = cap1.meet(&cap2).unwrap();

        // The meet capability should only allow reading in the intersection
        assert!(meet
            .permits(&AccessRequest::Memory {
                address: 1600,
                size: 100,
                read: true,
                write: false,
            })
            .is_ok());

        assert!(meet
            .permits(&AccessRequest::Memory {
                address: 1600,
                size: 100,
                read: false,
                write: true,
            })
            .is_err());

        // Should deny access outside the intersection
        assert!(meet
            .permits(&AccessRequest::Memory {
                address: 1200,
                size: 100,
                read: true,
                write: false,
            })
            .is_err());
    }
}
