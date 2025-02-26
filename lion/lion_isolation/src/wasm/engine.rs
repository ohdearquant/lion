//! WebAssembly engine.
//! 
//! This module provides a WebAssembly engine for plugin isolation.

use std::sync::Arc;
use anyhow::Result;
use wasmtime::{Engine, Config, Strategy, Module, Store, Instance, Linker, Memory, Caller};
use tracing::{debug, error, trace};

use crate::resource::{ResourceLimiter, ResourceMetering};
use crate::wasm::memory::WasmMemory;
use crate::wasm::module::WasmModule;
use crate::wasm::hostcall::HostCallContext;

/// A WebAssembly engine.
pub struct WasmEngine {
    /// The wasmtime engine.
    engine: Engine,
    
    /// The resource limiter.
    resource_limiter: Arc<dyn ResourceLimiter>,
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
        config.strategy(Strategy::Auto)?;
        
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
    pub fn default() -> Result<Self> {
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
        let mut linker = Linker::new(&self.engine);
        
        // Return the module
        Ok(WasmModule {
            module,
            linker,
        })
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
        store.add_fuel(u64::MAX / 2)?;
        
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
        linker: &mut Linker<T>,
        module: &str,
        functions: &[(&str, wasmtime::Func)],
    ) -> Result<()> {
        trace!("Adding host functions to module '{}'", module);
        
        for (name, func) in functions {
            linker.define(module, name, func.clone())?;
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
        let memory = instance.get_memory(store, memory_name)
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
        let func = instance.get_func(store, function_name)
            .ok_or_else(|| anyhow::anyhow!("Function not found"))?
            .typed(store)?;
        
        // Call the function
        let result = func.call(store, params)?;
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmtime::Global;
    
    #[test]
    fn test_compile_module() {
        // Create a simple WebAssembly module
        const WASM: &[u8] = include_bytes!("../../../tests/testdata/simple.wasm");
        
        // Create an engine
        let engine = WasmEngine::default().unwrap();
        
        // Compile the module
        let module = engine.compile_module(WASM).unwrap();
        
        // Check that the module was compiled
        assert!(module.module.imports().count() == 0);
    }
    
    #[test]
    fn test_instantiate_module() {
        // Create a simple WebAssembly module
        const WASM: &[u8] = include_bytes!("../../../tests/testdata/simple.wasm");
        
        // Create an engine
        let engine = WasmEngine::default().unwrap();
        
        // Compile the module
        let module = engine.compile_module(WASM).unwrap();
        
        // Create a host context
        let host_context = HostCallContext::new("test".to_string());
        
        // Instantiate the module
        let store = engine.instantiate_module(&module, host_context).unwrap();
        
        // Check that the store was created
        assert_eq!(store.data().plugin_id, "test");
    }
    
    #[test]
    fn test_add_host_functions() {
        // Create a simple WebAssembly module
        const WASM: &[u8] = include_bytes!("../../../tests/testdata/imports.wasm");
        
        // Create an engine
        let engine = WasmEngine::default().unwrap();
        
        // Compile the module
        let mut module = engine.compile_module(WASM).unwrap();
        
        // Define a global in the linker
        let global_type = wasmtime::GlobalType::new(wasmtime::ValType::I32, false);
        let global = Global::new(&engine.engine(), global_type, 42.into()).unwrap();
        
        module.linker.define("env", "global", global).unwrap();
        
        // Add a host function
        engine.register_function(
            &mut module.linker,
            "env",
            "function",
            |_: Caller<'_, HostCallContext>, param: i32| -> i32 {
                param + 1
            },
        ).unwrap();
        
        // Create a host context
        let host_context = HostCallContext::new("test".to_string());
        
        // Instantiate the module
        let mut store = engine.instantiate_module(&module, host_context).unwrap();
        
        // Get the instance
        let instance = module.linker.instantiate(&mut store, &module.module).unwrap();
        
        // Call the exported function
        let result: i32 = engine.call_function(
            &mut store,
            &instance,
            "call_imported_function",
            (42,),
        ).unwrap();
        
        // Check the result
        assert_eq!(result, 43);
        
        // Call the exported function that gets the global
        let result: i32 = engine.call_function(
            &mut store,
            &instance,
            "get_imported_global",
            (),
        ).unwrap();
        
        // Check the result
        assert_eq!(result, 42);
    }
}