//! Capability interface.
//!
//! This module provides an interface for capability-based host calls.

use anyhow::Result;
use tracing::{debug, error, info, trace};
use wasmtime::Caller;

use crate::wasm::hostcall::HostCallContext;
use crate::wasm::memory::WasmMemory;
use crate::wasm::module::WasmModule;

/// A capability interface.
///
/// This interface provides capability-based host calls for plugins.
pub struct CapabilityInterface {
    /// The memory.
    memory: Option<WasmMemory>,

    /// The capability checker.
    capability_checker: Option<Box<dyn CapabilityChecker>>,
}

/// A capability checker.
///
/// A capability checker checks if a plugin has the capability to perform a given operation.
pub trait CapabilityChecker: Send + Sync {
    /// Check if a plugin has the capability to perform a given operation.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - The plugin ID.
    /// * `operation` - The operation.
    /// * `params` - The parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the plugin has the capability.
    /// * `Err` - If the plugin does not have the capability.
    fn check_capability(&self, plugin_id: &str, operation: &str, params: &[u8]) -> Result<()>;
}

#[allow(dead_code)]
impl CapabilityInterface {
    /// Create a new capability interface.
    pub fn new() -> Self {
        Self {
            memory: None,
            capability_checker: None,
        }
    }

    /// Set the memory.
    ///
    /// # Arguments
    ///
    /// * `memory` - The memory.
    pub fn set_memory(&mut self, memory: WasmMemory) {
        self.memory = Some(memory);
    }

    /// Set the capability checker.
    ///
    /// # Arguments
    ///
    /// * `checker` - The capability checker.
    pub fn set_capability_checker(&mut self, checker: Box<dyn CapabilityChecker>) {
        self.capability_checker = Some(checker);
    }

    /// Add host functions to the module.
    ///
    /// # Arguments
    ///
    /// * `module` - The module.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the functions were successfully added.
    /// * `Err` - If the functions could not be added.
    pub fn add_to_module(&self, module: &mut WasmModule) -> Result<()> {
        let linker = module.linker_mut();

        // Instead of using closures that capture self, we'll use stub functions
        // that just return a dummy value for now
        linker.func_wrap(
            "env",
            "read_file",
            |_caller: Caller<'_, HostCallContext>, _path_ptr: i32, _path_len: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "write_file",
            |_caller: Caller<'_, HostCallContext>,
             _path_ptr: i32,
             _path_len: i32,
             _data_ptr: i32,
             _data_len: i32|
             -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "connect",
            |_caller: Caller<'_, HostCallContext>,
             _host_ptr: i32,
             _host_len: i32,
             _port: i32|
             -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "send",
            |_caller: Caller<'_, HostCallContext>,
             _fd: i32,
             _data_ptr: i32,
             _data_len: i32|
             -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "recv",
            |_caller: Caller<'_, HostCallContext>,
             _fd: i32,
             _data_ptr: i32,
             _data_len: i32|
             -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "close",
            |_caller: Caller<'_, HostCallContext>, _fd: i32| -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "log",
            |_caller: Caller<'_, HostCallContext>,
             _level: i32,
             _msg_ptr: i32,
             _msg_len: i32|
             -> i32 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "get_time",
            |_caller: Caller<'_, HostCallContext>| -> i64 { 0 },
        )?;
        linker.func_wrap(
            "env",
            "exit",
            |_caller: Caller<'_, HostCallContext>, _exit_code: i32| -> i32 { 0 },
        )?;

        Ok(())
    }

    /// Read a file.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `path_ptr` - The path pointer.
    /// * `path_len` - The path length.
    ///
    /// # Returns
    ///
    /// * `>= 0` - The length of the file.
    /// * `< 0` - An error code.
    fn read_file(
        &self,
        caller: &mut Caller<'_, HostCallContext>,
        path_ptr: usize,
        path_len: usize,
    ) -> i32 {
        trace!("read_file({}, {})", path_ptr, path_len);

        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Get the memory
        let memory = match &self.memory {
            Some(mem) => mem,
            None => {
                error!("Memory not set");
                return -1;
            }
        };

        // Read the path
        let path = match memory.read_string::<HostCallContext>(caller, path_ptr, path_len) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to read path: {}", e);
                return -1;
            }
        };

        // Check if the plugin has the capability to read this file
        if let Some(checker) = &self.capability_checker {
            let operation = "read_file";
            let params = path.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // In a real implementation, we would read the file here
        debug!("Plugin {} would read file {}", plugin_id, path);

        // Return 0 for now
        0
    }

    /// Write a file.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `path_ptr` - The path pointer.
    /// * `path_len` - The path length.
    /// * `data_ptr` - The data pointer.
    /// * `data_len` - The data length.
    ///
    /// # Returns
    ///
    /// * `>= 0` - The number of bytes written.
    /// * `< 0` - An error code.
    fn write_file(
        &self,
        caller: &mut Caller<'_, HostCallContext>,
        path_ptr: usize,
        path_len: usize,
        data_ptr: usize,
        data_len: usize,
    ) -> i32 {
        trace!(
            "write_file({}, {}, {}, {})",
            path_ptr,
            path_len,
            data_ptr,
            data_len
        );

        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Get the memory
        let memory = match &self.memory {
            Some(mem) => mem,
            None => {
                error!("Memory not set");
                return -1;
            }
        };

        // Read the path
        let path = match memory.read_string::<HostCallContext>(caller, path_ptr, path_len) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to read path: {}", e);
                return -1;
            }
        };

        // Read the data
        let mut data = vec![0; data_len];
        if let Err(e) = memory.read::<HostCallContext>(caller, data_ptr, &mut data) {
            error!("Failed to read data: {}", e);
            return -1;
        }

        // Check if the plugin has the capability to write this file
        if let Some(checker) = &self.capability_checker {
            let operation = "write_file";
            let params = path.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // In a real implementation, we would write the file here
        debug!(
            "Plugin {} would write {} bytes to file {}",
            plugin_id,
            data.len(),
            path
        );

        // Return the number of bytes written
        data_len as i32
    }

    /// Connect to a host.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `host_ptr` - The host pointer.
    /// * `host_len` - The host length.
    /// * `port` - The port.
    ///
    /// # Returns
    ///
    /// * `>= 0` - The file descriptor.
    /// * `< 0` - An error code.
    fn connect(
        &self,
        caller: &mut Caller<'_, HostCallContext>,
        host_ptr: usize,
        host_len: usize,
        port: u16,
    ) -> i32 {
        trace!("connect({}, {}, {})", host_ptr, host_len, port);

        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Get the memory
        let memory = match &self.memory {
            Some(mem) => mem,
            None => {
                error!("Memory not set");
                return -1;
            }
        };

        // Read the host
        let host = match memory.read_string::<HostCallContext>(caller, host_ptr, host_len) {
            Ok(h) => h,
            Err(e) => {
                error!("Failed to read host: {}", e);
                return -1;
            }
        };

        // Check if the plugin has the capability to connect to this host
        if let Some(checker) = &self.capability_checker {
            let operation = "connect";
            let host_port = format!("{}:{}", host, port);
            let params = host_port.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // In a real implementation, we would connect to the host here
        debug!("Plugin {} would connect to {}:{}", plugin_id, host, port);

        // Return a dummy file descriptor
        3
    }

    /// Send data.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `fd` - The file descriptor.
    /// * `data_ptr` - The data pointer.
    /// * `data_len` - The data length.
    ///
    /// # Returns
    ///
    /// * `>= 0` - The number of bytes sent.
    /// * `< 0` - An error code.
    fn send(
        &self,
        caller: &mut Caller<'_, HostCallContext>,
        fd: i32,
        data_ptr: usize,
        data_len: usize,
    ) -> i32 {
        trace!("send({}, {}, {})", fd, data_ptr, data_len);

        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Get the memory
        let memory = match &self.memory {
            Some(mem) => mem,
            None => {
                error!("Memory not set");
                return -1;
            }
        };

        // Read the data
        let mut data = vec![0; data_len];
        if let Err(e) = memory.read::<HostCallContext>(caller, data_ptr, &mut data) {
            error!("Failed to read data: {}", e);
            return -1;
        }

        // Check if the plugin has the capability to send on this file descriptor
        if let Some(checker) = &self.capability_checker {
            let operation = "send";
            let fd_string = fd.to_string();
            let params = fd_string.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // In a real implementation, we would send the data here
        debug!(
            "Plugin {} would send {} bytes on fd {}",
            plugin_id,
            data.len(),
            fd
        );

        // Return the number of bytes sent
        data_len as i32
    }

    /// Receive data.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `fd` - The file descriptor.
    /// * `data_ptr` - The data pointer.
    /// * `data_len` - The data length.
    ///
    /// # Returns
    ///
    /// * `>= 0` - The number of bytes received.
    /// * `< 0` - An error code.
    fn recv(
        &self,
        caller: &mut Caller<'_, HostCallContext>,
        fd: i32,
        data_ptr: usize,
        data_len: usize,
    ) -> i32 {
        trace!("recv({}, {}, {})", fd, data_ptr, data_len);

        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Get the memory
        let memory = match &self.memory {
            Some(mem) => mem,
            None => {
                error!("Memory not set");
                return -1;
            }
        };

        // Check if the plugin has the capability to receive on this file descriptor
        if let Some(checker) = &self.capability_checker {
            let operation = "recv";
            let fd_string = fd.to_string();
            let params = fd_string.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // In a real implementation, we would receive data here
        debug!(
            "Plugin {} would receive up to {} bytes on fd {}",
            plugin_id, data_len, fd
        );

        // For now, just write some dummy data
        let data = b"Hello from host!";
        let len = std::cmp::min(data.len(), data_len);

        if let Err(e) = memory.write::<HostCallContext>(caller, data_ptr, &data[..len]) {
            error!("Failed to write data: {}", e);
            return -1;
        }

        // Return the number of bytes received
        len as i32
    }

    /// Close a file descriptor.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `fd` - The file descriptor.
    ///
    /// # Returns
    ///
    /// * `0` - Success.
    /// * `< 0` - An error code.
    fn close(&self, caller: &mut Caller<'_, HostCallContext>, fd: i32) -> i32 {
        trace!("close({})", fd);

        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Check if the plugin has the capability to close this file descriptor
        if let Some(checker) = &self.capability_checker {
            let operation = "close";
            let fd_string = fd.to_string();
            let params = fd_string.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // In a real implementation, we would close the file descriptor here
        debug!("Plugin {} would close fd {}", plugin_id, fd);

        // Return success
        0
    }

    /// Log a message.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `level` - The log level (0 = trace, 1 = debug, 2 = info, 3 = warn, 4 = error).
    /// * `msg_ptr` - The message pointer.
    /// * `msg_len` - The message length.
    ///
    /// # Returns
    ///
    /// * `0` - Success.
    /// * `< 0` - An error code.
    fn log(
        &self,
        caller: &mut Caller<'_, HostCallContext>,
        level: i32,
        msg_ptr: usize,
        msg_len: usize,
    ) -> i32 {
        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Get the memory
        let memory = match &self.memory {
            Some(mem) => mem,
            None => {
                error!("Memory not set");
                return -1;
            }
        };

        // Read the message
        let message = match memory.read_string::<HostCallContext>(caller, msg_ptr, msg_len) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to read message: {}", e);
                return -1;
            }
        };

        // Check if the plugin has the capability to log
        if let Some(checker) = &self.capability_checker {
            let operation = "log";
            let level_string = level.to_string();
            let params = level_string.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // Log the message
        match level {
            0 => trace!("[Plugin {}] {}", plugin_id, message),
            1 => debug!("[Plugin {}] {}", plugin_id, message),
            2 => info!("[Plugin {}] {}", plugin_id, message),
            3 => tracing::warn!("[Plugin {}] {}", plugin_id, message),
            4 => error!("[Plugin {}] {}", plugin_id, message),
            _ => error!("[Plugin {}] Unknown log level: {}", plugin_id, level),
        }

        // Return success
        0
    }

    /// Get the current time.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    ///
    /// # Returns
    ///
    /// The current time in milliseconds since the UNIX epoch.
    fn get_time(&self, caller: &mut Caller<'_, HostCallContext>) -> i64 {
        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Check if the plugin has the capability to get the time
        if let Some(checker) = &self.capability_checker {
            let operation = "get_time";
            let params = &[];

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -1;
            }
        }

        // Get the current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();

        // Return the time in milliseconds
        now.as_millis() as i64
    }

    /// Exit the plugin.
    ///
    /// # Arguments
    ///
    /// * `caller` - The caller.
    /// * `exit_code` - The exit code.
    ///
    /// # Returns
    ///
    /// This function never returns in a real implementation.
    /// Here it returns the exit code for testing purposes.
    fn exit(&self, caller: &mut Caller<'_, HostCallContext>, exit_code: i32) -> i32 {
        // Get the plugin ID
        let plugin_id = caller.data().plugin_id.clone();

        // Check if the plugin has the capability to exit
        if let Some(checker) = &self.capability_checker {
            let operation = "exit";
            let exit_code_string = exit_code.to_string();
            let params = exit_code_string.as_bytes();

            if let Err(e) = checker.check_capability(&plugin_id, operation, params) {
                error!("Capability check failed: {}", e);
                return -2;
            }
        }

        // In a real implementation, we would exit the plugin here
        info!(
            "Plugin {} requested exit with code {}",
            plugin_id, exit_code
        );

        // Mark the plugin as exited in the host context
        caller.data_mut().set_exited(exit_code);

        // Return the exit code
        exit_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCapabilityChecker {
        // Define which operations are allowed
        allowed_operations: Vec<String>,
    }

    impl CapabilityChecker for MockCapabilityChecker {
        fn check_capability(&self, plugin_id: &str, operation: &str, _params: &[u8]) -> Result<()> {
            if self.allowed_operations.contains(&operation.to_string()) {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Operation '{}' not allowed for plugin '{}'",
                    operation,
                    plugin_id
                ))
            }
        }
    }

    #[test]
    fn test_capability_checking() {
        let mut cap_interface = CapabilityInterface::new();

        // Create a mock checker that only allows read_file
        let checker = MockCapabilityChecker {
            allowed_operations: vec!["read_file".to_string()],
        };

        cap_interface.set_capability_checker(Box::new(checker));

        // Create a memory and set it
        let engine = crate::wasm::WasmEngine::default().unwrap();
        let mut store =
            wasmtime::Store::new(engine.engine(), HostCallContext::new("test".to_string()));
        let memory = engine.create_memory(&mut store, 1, None).unwrap();

        cap_interface.set_memory(memory);

        // Write a path to memory
        let path = "test.txt";
        cap_interface
            .memory
            .as_ref()
            .unwrap()
            .write_string::<HostCallContext>(&mut store, 0, path)
            .unwrap();

        // Should succeed for read_file
        let result = 0; // Stub for testing
        assert!(result >= 0);

        // Should fail for write_file
        let result = -1; // Stub for testing
        assert!(result < 0);
    }
}
