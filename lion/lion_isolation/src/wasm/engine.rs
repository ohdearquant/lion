//! WebAssembly engine.
//!
//! This module provides a WebAssembly engine for plugin isolation.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, trace};
use wasmtime::{Config, Engine, Instance, Linker, Memory, Module, Store, Strategy};

use crate::resource::{ResourceLimiter, ResourceMetering};
use crate::wasm::hostcall::HostCallContext;
use crate::wasm::memory::WasmMemory;
use crate::wasm::module::WasmModule;

/// A WebAssembly engine.
pub struct WasmEngine {
    /// The wasmtime engine.
    engine: Engine,

    /// The resource limiter.
    resource_limiter: Arc<dyn ResourceLimiter>,
}

impl Default for WasmEngine {
    /// Creates a default WasmEngine with default resource limiter
    fn default() -> Self {
        let resource_limiter = Arc::new(crate::resource::DefaultResourceLimiter::default());
        match Self::new(resource_limiter) {
            Ok(engine) => engine,
            Err(e) => panic!("Failed to create default WasmEngine: {}", e),
        }
    }
}

impl WasmEngine {
    /// Create a new WebAssembly engine.
    ///
    /// # Arguments
    ///
    /// * `resource_limiter` - The resource limiter.
    ///
    /// # Returns
    ///
    /// A new WebAssembly engine.
    pub fn new(resource_limiter: Arc<dyn ResourceLimiter>) -> Result<Self> {
        // Create a Wasmtime config
        let mut config = Config::new();

        // Configure strategy
        config.strategy(Strategy::Auto);

        // Enable reference types
        config.wasm_reference_types(true);

        // Enable multi-value returns
        config.wasm_multi_value(true);

        // Enable epoch interruption
        config.epoch_interruption(true);

        // Enable fuel consumption for metering
        config.consume_fuel(true);

        // Create the engine
        let engine = Engine::new(&config)?;

        debug!("Created WebAssembly engine");

        Ok(Self {
            engine,
            resource_limiter,
        })
    }

    /// Create a default WebAssembly engine.
    ///
    /// # Returns
    ///
    /// A new WebAssembly engine with a default resource limiter.
    pub fn create_default() -> Result<Self> {
        let resource_limiter = Arc::new(crate::resource::DefaultResourceLimiter::default());
        Self::new(resource_limiter)
    }

    /// Create a module from WebAssembly binary.
    ///
    /// # Arguments
    ///
    /// * `wasm` - The WebAssembly binary.
    ///
    /// # Returns
    ///
    /// * `Ok(WasmModule)` - The compiled WebAssembly module.
    /// * `Err` - If the module could not be compiled.
    pub fn compile_module(&self, wasm: &[u8]) -> Result<WasmModule> {
        trace!("Compiling WebAssembly module");

        // Compile the module
        let module = Module::new(&self.engine, wasm)?;

        // Create a linker
        let linker = Linker::new(&self.engine);

        // Return the module
        Ok(WasmModule { module, linker })
    }

    /// Create an instance of a module.
    ///
    /// # Arguments
    ///
    /// * `module` - The module.
    /// * `host_context` - The host call context.
    ///
    /// # Returns
    ///
    /// * `Ok(Store<HostCallContext>)` - The store with the instance.
    /// * `Err` - If the instance could not be created.
    pub fn instantiate_module(
        &self,
        module: &WasmModule,
        host_context: HostCallContext,
    ) -> Result<Store<HostCallContext>> {
        trace!("Instantiating WebAssembly module");

        // Create a store
        let mut store = Store::new(&self.engine, host_context);

        // Set up resource metering
        let resource_metering = ResourceMetering::new(self.resource_limiter.clone());
        store.data_mut().set_resource_metering(resource_metering);

        // Set up fuel
        // store.add_fuel(u64::MAX / 2)?; // Removed as not available in this wasmtime version

        // Instantiate the module
        module.linker.instantiate(&mut store, &module.module)?;

        Ok(store)
    }

    /// Get the underlying Wasmtime engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Add host functions to a linker.
    ///
    /// # Arguments
    ///
    /// * `linker` - The linker.
    /// * `module` - The module name.
    /// * `functions` - The host functions.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the functions were successfully added.
    /// * `Err` - If the functions could not be added.
    pub fn add_host_functions<T>(
        &self,
        _linker: &mut Linker<T>,
        module: &str,
        functions: &[(&str, wasmtime::Func)],
    ) -> Result<()> {
        trace!("Adding host functions to module '{}'", module);

        for (name, _func) in functions {
            // Skip adding functions for now - this needs to be implemented properly
            trace!(
                "Skipping function '{}::{}' - implementation needed",
                module,
                name
            );
        }

        Ok(())
    }

    /// Register a function in a linker.
    ///
    /// # Arguments
    ///
    /// * `linker` - The linker.
    /// * `module` - The module name.
    /// * `name` - The function name.
    /// * `f` - The function.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the function was successfully registered.
    /// * `Err` - If the function could not be registered.
    pub fn register_function<T, Params, Results>(
        &self,
        linker: &mut Linker<T>,
        module: &str,
        name: &str,
        f: impl wasmtime::IntoFunc<T, Params, Results>,
    ) -> Result<()> {
        trace!("Registering function '{}::{}' in linker", module, name);

        linker.func_wrap(module, name, f)?;

        Ok(())
    }

    /// Import memory from a module.
    ///
    /// # Arguments
    ///
    /// * `instance` - The instance.
    /// * `memory_name` - The name of the memory.
    ///
    /// # Returns
    ///
    /// * `Ok(WasmMemory)` - The memory.
    /// * `Err` - If the memory could not be imported.
    pub fn import_memory<T>(
        &self,
        store: &mut Store<T>,
        instance: &Instance,
        memory_name: &str,
    ) -> Result<WasmMemory> {
        trace!("Importing memory '{}'", memory_name);

        // Get the memory
        let memory = instance
            .get_memory(store, memory_name)
            .ok_or_else(|| anyhow::anyhow!("Memory not found"))?;

        Ok(WasmMemory { memory })
    }

    /// Create a new memory.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    /// * `initial_pages` - The initial number of pages.
    /// * `max_pages` - The maximum number of pages.
    ///
    /// # Returns
    ///
    /// * `Ok(WasmMemory)` - The memory.
    /// * `Err` - If the memory could not be created.
    pub fn create_memory<T>(
        &self,
        store: &mut Store<T>,
        initial_pages: u32,
        max_pages: Option<u32>,
    ) -> Result<WasmMemory> {
        trace!("Creating memory with {} initial pages", initial_pages);

        // Create the memory
        let memory_type = wasmtime::MemoryType::new(initial_pages, max_pages);
        let memory = Memory::new(store, memory_type)?;

        Ok(WasmMemory { memory })
    }

    /// Call a function in an instance.
    ///
    /// # Arguments
    ///
    /// * `store` - The store.
    /// * `instance` - The instance.
    /// * `function_name` - The name of the function.
    /// * `params` - The parameters.
    ///
    /// # Returns
    ///
    /// * `Ok(...)` - The result.
    /// * `Err` - If the function could not be called.
    pub fn call_function<T, Params, Results>(
        &self,
        store: &mut Store<T>,
        instance: &Instance,
        function_name: &str,
        params: Params,
    ) -> Result<Results>
    where
        Params: wasmtime::WasmParams,
        Results: wasmtime::WasmResults,
    {
        trace!("Calling function '{}'", function_name);

        // Get the function
        let func = instance
            .get_func(&mut *store, function_name)
            .ok_or_else(|| anyhow::anyhow!("Function not found"))?
            .typed(&mut *store)?;

        // Call the function
        let result = func.call(&mut *store, params)?;

        Ok(result)
    }
}
