//! WebAssembly memory.
//!
//! This module provides a wrapper for WebAssembly memory.

use wasmtime::Memory;

/// A WebAssembly memory.
pub struct WasmMemory {
    /// The wasmtime memory.
    pub(crate) memory: Memory,
}

impl WasmMemory {
    /// Get the underlying Wasmtime memory.
    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    /// Read from memory.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    /// * `offset` - The offset.
    /// * `data` - The buffer to read into.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the memory was successfully read.
    /// * `Err` - If the memory could not be read.
    pub fn read<T>(
        &self,
        store: &impl wasmtime::AsContextMut,
        offset: usize,
        data: &mut [u8],
    ) -> Result<(), anyhow::Error> {
        // Check bounds
        if offset + data.len() > self.memory.data_size(store) {
            return Err(anyhow::anyhow!("Out of bounds memory access"));
        }

        // Read from memory
        let memory_slice = &self.memory.data(store)[offset..offset + data.len()];
        data.copy_from_slice(memory_slice);

        Ok(())
    }

    /// Write to memory.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    /// * `offset` - The offset.
    /// * `data` - The data to write.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the memory was successfully written.
    /// * `Err` - If the memory could not be written.
    pub fn write<T>(
        &self,
        store: &mut impl wasmtime::AsContextMut,
        offset: usize,
        data: &[u8],
    ) -> Result<(), anyhow::Error> {
        // Check bounds
        if offset + data.len() > self.memory.data_size(&mut *store) {
            return Err(anyhow::anyhow!("Out of bounds memory access"));
        }

        // Write to memory
        let memory_slice = &mut self.memory.data_mut(&mut *store)[offset..offset + data.len()];
        memory_slice.copy_from_slice(data);

        Ok(())
    }

    /// Read a string from memory.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    /// * `offset` - The offset.
    /// * `length` - The length of the string.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The string.
    /// * `Err` - If the string could not be read.
    pub fn read_string<T>(
        &self,
        store: &impl wasmtime::AsContextMut,
        offset: usize,
        length: usize,
    ) -> Result<String, anyhow::Error> {
        // Read the string
        let mut data = vec![0; length];
        self.read::<T>(store, offset, &mut data)?;

        // Convert to string
        let string = String::from_utf8(data)?;

        Ok(string)
    }

    /// Write a string to memory.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    /// * `offset` - The offset.
    /// * `string` - The string to write.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the string was successfully written.
    /// * `Err` - If the string could not be written.
    pub fn write_string<T>(
        &self,
        store: &mut impl wasmtime::AsContextMut,
        offset: usize,
        string: &str,
    ) -> Result<(), anyhow::Error> {
        // Write the string
        self.write::<T>(store, offset, string.as_bytes())
    }

    /// Read a null-terminated string from memory.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    /// * `offset` - The offset.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The string.
    /// * `Err` - If the string could not be read.
    pub fn read_null_terminated_string<T>(
        &self,
        store: &impl wasmtime::AsContextMut,
        offset: usize,
    ) -> Result<String, anyhow::Error> {
        // Find the null terminator
        let memory_data = self.memory.data(store);
        let mut length = 0;

        while offset + length < memory_data.len() && memory_data[offset + length] != 0 {
            length += 1;
        }

        // Read the string
        self.read_string::<T>(store, offset, length)
    }

    /// Get the size of the memory.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    ///
    /// # Returns
    ///
    /// The size of the memory in bytes.
    pub fn size<T>(&self, store: &impl wasmtime::AsContextMut) -> usize {
        self.memory.data_size(store)
    }
}

#[cfg(test)]
mod tests {
    use crate::wasm::engine::WasmEngine;
    use crate::{wasm::hostcall::HostCallContext, wasm::memory::WasmMemory};

    #[test]
    fn test_read_write_memory() {
        // Create an engine
        let engine = WasmEngine::default().unwrap();

        // Create a host context
        let host_context = HostCallContext::new("test".to_string());

        // Create a store
        let mut store = wasmtime::Store::new(engine.engine(), host_context);

        // Create a memory
        let memory = engine.create_memory(&mut store, 1, None).unwrap();

        // Write to memory
        memory
            .write::<HostCallContext>(&mut store, 0, b"hello")
            .unwrap();

        // Read from memory
        let mut data = [0; 5];
        memory
            .read::<HostCallContext>(&store, 0, &mut data)
            .unwrap();

        // Check the data
        assert_eq!(data, *b"hello");
    }

    #[test]
    fn test_read_write_string() {
        // Create an engine
        let engine = WasmEngine::default().unwrap();

        // Create a host context
        let host_context = HostCallContext::new("test".to_string());

        // Create a store
        let mut store = wasmtime::Store::new(engine.engine(), host_context);

        // Create a memory
        let memory = engine.create_memory(&mut store, 1, None).unwrap();

        // Write a string to memory
        memory
            .write_string::<HostCallContext>(&mut store, 0, "hello")
            .unwrap();

        // Read the string from memory
        let string = memory.read_string::<HostCallContext>(&store, 0, 5).unwrap();

        // Check the string
        assert_eq!(string, "hello");
    }

    #[test]
    fn test_read_null_terminated_string() {
        // Create an engine
        let engine = WasmEngine::default().unwrap();

        // Create a host context
        let host_context = HostCallContext::new("test".to_string());

        // Create a store
        let mut store = wasmtime::Store::new(engine.engine(), host_context);

        // Create a memory
        let memory = engine.create_memory(&mut store, 1, None).unwrap();

        // Write a null-terminated string to memory
        let data = b"hello\0world";
        memory
            .write::<HostCallContext>(&mut store, 0, data)
            .unwrap();

        // Read the null-terminated string from memory
        let string = memory
            .read_null_terminated_string::<HostCallContext>(&store, 0)
            .unwrap();

        // Check the string
        assert_eq!(string, "hello");
    }
}
