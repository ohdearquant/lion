//! # Collections
//! 
//! This module provides specialized concurrent data structures used throughout the Lion framework.
//! 
//! ## Components
//! 
//! - `Pile`: A thread-safe, ordered collection that maintains insertion order and supports
//!   concurrent access. Used primarily for message queuing and event storage.
//! 
//! - `Progression`: A concurrent data structure for tracking ordered sequences of events or
//!   states, particularly useful for monitoring task progression and agent state changes.
//!
//! Both collections are designed to be thread-safe and support the concurrent nature of
//! the Lion framework's multi-agent architecture.

mod pile;
mod progression;

pub use pile::Pile;
pub use progression::Progression;