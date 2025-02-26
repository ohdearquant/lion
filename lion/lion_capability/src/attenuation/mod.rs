//! Capability attenuation.
//! 
//! This module provides capability attenuation functionality.

pub mod filter;
pub mod proxy;
pub mod combine;

pub use filter::FilterCapability;
pub use proxy::ProxyCapability;
pub use combine::CombineCapability;