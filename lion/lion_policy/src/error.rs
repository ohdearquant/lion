//! Error handling for the Lion policy system.
//!
//! This module provides error conversion functions to handle
//! errors from dependent crates.

use lion_capability::CapabilityError as CapabilityLibError;
use lion_core::error::{CapabilityError, Error};

/// Convert a lion_capability::CapabilityError to lion_core::error::Error
pub fn capability_error_to_core_error(err: CapabilityLibError) -> Error {
    Error::Capability(CapabilityError::ConstraintError(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_error_conversion() {
        let cap_err = CapabilityLibError::AccessDenied("Test error".to_string());
        let error = capability_error_to_core_error(cap_err);
        assert!(matches!(
            error,
            Error::Capability(CapabilityError::ConstraintError(_))
        ));
    }
}
