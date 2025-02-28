//! Convenience macros for the Lion microkernel.
//!
//! This module provides a set of macros that are useful throughout the
//! system for common operations like logging, error handling, and capability
//! checking.

/// Log an event with the given level and module.
///
/// This macro provides a convenient way to log events with structured
/// metadata, including the current module, file, and line number.
///
/// # Examples
///
/// ```
/// use lion_core::log_event;
/// use lion_core::utils::LogLevel;
///
/// // Log an info message
/// log_event!(LogLevel::Info, "Hello, world!");
///
/// // Log a warning with additional context
/// log_event!(LogLevel::Warning, "Resource usage high",
///     resource_type => "memory",
///     usage_percent => 85,
///     threshold => 80,
/// );
/// ```
#[macro_export]
macro_rules! log_event {
    ($level:expr, $message:expr) => {
        {
            use $crate::utils::LogLevel;
            match $level {
                LogLevel::Error => log::error!("[{}] {}", module_path!(), $message),
                LogLevel::Warning => log::warn!("[{}] {}", module_path!(), $message),
                LogLevel::Info => log::info!("[{}] {}", module_path!(), $message),
                LogLevel::Debug => log::debug!("[{}] {}", module_path!(), $message),
                LogLevel::Trace => log::trace!("[{}] {}", module_path!(), $message),
            }
        }
    };

    ($level:expr, $message:expr, $($key:ident => $value:expr),+ $(,)?) => {
        {
            use $crate::utils::LogLevel;
            let metadata = vec![$(format!("{}={}", stringify!($key), $value)),+].join(" ");
            match $level {
                LogLevel::Error => log::error!("[{}] {}: {}", module_path!(), $message, metadata),
                LogLevel::Warning => log::warn!("[{}] {}: {}", module_path!(), $message, metadata),
                LogLevel::Info => log::info!("[{}] {}: {}", module_path!(), $message, metadata),
                LogLevel::Debug => log::debug!("[{}] {}: {}", module_path!(), $message, metadata),
                LogLevel::Trace => log::trace!("[{}] {}: {}", module_path!(), $message, metadata),
            }
        }
    };
}

/// Check if the given capability permits the access request.
///
/// This macro provides a convenient way to check if a capability permits
/// a given access request, with automatic error handling.
///
/// # Examples
///
/// ```
/// use lion_core::{check_capability, traits::Capability, types::AccessRequest};
///
/// fn do_something(cap: &impl Capability, path: &str) -> lion_core::Result<()> {
///     let request = AccessRequest::file_read(path);
///     
///     // This will return early with an error if the capability doesn't permit the access
///     check_capability!(cap, &request);
///     
///     // Continue with the operation if permitted
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! check_capability {
    ($capability:expr, $request:expr) => {
        match $capability.permits($request) {
            Ok(()) => {}
            Err(e) => return Err(e.into()),
        }
    };

    ($capability:expr, $request:expr, $error_msg:expr) => {
        match $capability.permits($request) {
            Ok(()) => {}
            Err(e) => {
                log::warn!(
                    "[{}] Capability check failed: {} - {}",
                    module_path!(),
                    $error_msg,
                    e
                );
                return Err(e.into());
            }
        }
    };
}

/// Execute a block of code with a capability check.
///
/// This macro provides a convenient way to execute a block of code only
/// if a capability permits a given access request.
///
/// # Examples
///
/// ```
/// use lion_core::{with_capability, traits::Capability, types::AccessRequest};
///
/// fn read_file(cap: &impl Capability, path: &str) -> lion_core::Result<String> {
///     let request = AccessRequest::file_read(path);
///     
///     with_capability!(cap, &request, {
///         // This code will only execute if the capability permits the access
///         std::fs::read_to_string(path).map_err(|e| e.into())
///     })
/// }
/// ```
#[macro_export]
macro_rules! with_capability {
    ($capability:expr, $request:expr, $block:block) => {
        match $capability.permits($request) {
            Ok(()) => $block,
            Err(e) => Err(e.into()),
        }
    };

    ($capability:expr, $request:expr, $block:block, $error_msg:expr) => {
        match $capability.permits($request) {
            Ok(()) => $block,
            Err(e) => {
                log::warn!(
                    "[{}] Capability check failed: {} - {}",
                    module_path!(),
                    $error_msg,
                    e
                );
                Err(e.into())
            }
        }
    };
}

/// Log and wrap an error.
///
/// This macro provides a convenient way to log an error and wrap it
/// in a new error type.
///
/// # Examples
///
/// ```
/// use lion_core::{wrap_err, error::Error};
/// use std::io;
/// use std::path::PathBuf;
///
/// fn load_plugin(path: &str) -> lion_core::Result<()> {
///     let result = std::fs::read(path);
///     
///     // This will log the error and wrap it in an Error
///     let content = wrap_err!(result,
///                           Error::Io(std::io::Error::new(
///                               std::io::ErrorKind::NotFound,
///                               format!("Failed to read plugin file: {}", path)
///                           )));
///     
///     // Continue with the operation if successful
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! wrap_err {
    ($result:expr, $error:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => {
                log::error!("[{}:{}] {}: {}", file!(), line!(), stringify!($error), e);
                return Err($crate::Error::from($error));
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::error::{CapabilityError, Result};
    use crate::traits::Capability;
    use crate::types::AccessRequest;

    struct TestCapability {
        permitted: bool,
    }

    impl Capability for TestCapability {
        fn capability_type(&self) -> &str {
            "test"
        }

        fn permits(&self, _request: &AccessRequest) -> std::result::Result<(), crate::Error> {
            if self.permitted {
                Ok(())
            } else {
                Err(CapabilityError::PermissionDenied("Test denial".into()).into())
            }
        }
    }

    #[test]
    fn test_check_capability_macro() {
        let permitted_cap = TestCapability { permitted: true };
        let denied_cap = TestCapability { permitted: false };
        let request = AccessRequest::file_read("/test");

        // Should not panic
        let result = (|| -> Result<()> {
            check_capability!(permitted_cap, &request);
            Ok(())
        })();
        assert!(result.is_ok());

        // Should return error
        let result = (|| -> Result<()> {
            check_capability!(denied_cap, &request);
            Ok(())
        })();
        assert!(result.is_err());
    }

    #[test]
    fn test_with_capability_macro() {
        let permitted_cap = TestCapability { permitted: true };
        let denied_cap = TestCapability { permitted: false };
        let request = AccessRequest::file_read("/test");

        // Should execute block and return success
        let result: Result<i32> = with_capability!(permitted_cap, &request, { Ok(42) });
        assert_eq!(result.unwrap(), 42);

        // Should not execute block and return error
        let result: Result<i32> = with_capability!(denied_cap, &request, { Ok(42) });
        assert!(result.is_err());
    }

    #[test]
    fn test_wrap_err_macro() {
        let ok_result: std::result::Result<i32, std::io::Error> = Ok(42);

        // Should return the value
        let result = (|| -> Result<i32> {
            let val = wrap_err!(ok_result, CapabilityError::NotGranted("Test".into()));
            Ok(val)
        })();
        assert_eq!(result.unwrap(), 42);

        let err_result: std::result::Result<i32, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Test IO error",
        ));

        // Should wrap the error
        let result = (|| -> Result<i32> {
            let val = wrap_err!(err_result, CapabilityError::NotGranted("Test".into()));
            Ok(val)
        })();
        assert!(result.is_err());
        match result {
            Err(e) => match e {
                crate::Error::Capability(_) => {}
                _ => panic!("Wrong error type"),
            },
            _ => panic!("Expected error"),
        }
    }

    #[test]
    fn test_log_event_macro() {
        // These are mostly compile-time tests
        log_event!(LogLevel::Info, "Test message");
        log_event!(LogLevel::Warning, "Test message with fields",
                 field1 => "value1",
                 field2 => 42,
        );
    }
}
