use wasmer::{imports, Function, Instance, Memory, Module, Store, TypedFunction};
use super::types::{ExecutionError, CosmWasmEnv};
use std::sync::{Arc, Mutex};

pub struct WasmerExecutor {
    store: Store,
    storage_prefix: Vec<u8>,
    env: Option<CosmWasmEnv>,
    state_changes: Arc<Mutex<Vec<(Vec<u8>, Option<Vec<u8>>)>>>,
    events: Arc<Mutex<Vec<(String, Vec<(String, String)>)>>>,
}

impl WasmerExecutor {
    pub fn new(storage_prefix: Vec<u8>) -> Self {
        Self {
            store: Store::default(),
            storage_prefix,
            env: None,
            state_changes: Arc::new(Mutex::new(Vec::new())),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set_env(&mut self, env: CosmWasmEnv) {
        self.env = Some(env);
    }

    pub fn execute_wasm(
        &mut self,
        wasm_code: &[u8],
        function_name: &str,
        args: &[u8],
    ) -> Result<Vec<u8>, ExecutionError> {
        // Validate WASM module
        if wasm_code.len() < 8 {
            return Err(ExecutionError::InvalidInput("WASM module too small".to_string()));
        }
        
        // Check WASM magic number and version
        if &wasm_code[0..4] != b"\0asm" || &wasm_code[4..8] != b"\x01\x00\x00\x00" {
            return Err(ExecutionError::InvalidInput("Invalid WASM module format".to_string()));
        }

        // Compile the module
        let module = match Module::new(&self.store, wasm_code) {
            Ok(m) => m,
            Err(e) => {
                // Handle compilation errors gracefully
                return Err(ExecutionError::CompilationError(format!("Failed to compile WASM: {}", e)));
            }
        };
        
        // Create import object with host functions
        let import_object = self.create_imports()?;
        
        // Instantiate the module
        let instance = Instance::new(&mut self.store, &module, &import_object)
            .map_err(|e| ExecutionError::InstantiationError(format!("Failed to instantiate module: {}", e)))?;
        
        // Get memory export
        let memory = instance.exports.get_memory("memory")
            .map_err(|e| ExecutionError::RuntimeError(format!("Failed to get memory export: {}", e)))?;
        
        // Try CosmWasm-style execution first
        if let Ok(result) = self.execute_cosmwasm_function(&instance, memory, function_name, args) {
            return Ok(result);
        }
        
        // Fallback to simple function call
        self.execute_simple_function(&instance, function_name)
    }

    fn execute_cosmwasm_function(
        &mut self,
        instance: &Instance,
        memory: &Memory,
        function_name: &str,
        args: &[u8],
    ) -> Result<Vec<u8>, ExecutionError> {
        // Get allocate function
        let allocate: TypedFunction<u32, u32> = instance
            .exports
            .get_typed_function(&self.store, "allocate")
            .map_err(|_| ExecutionError::RuntimeError("No allocate function found".to_string()))?;
        
        // Allocate memory for arguments
        let args_len = args.len() as u32;
        let args_ptr = allocate.call(&mut self.store, args_len)
            .map_err(|e| ExecutionError::RuntimeError(format!("Failed to allocate memory: {}", e)))?;
        
        // Write arguments to memory
        let view = memory.view(&self.store);
        view.write(args_ptr as u64, args)
            .map_err(|e| ExecutionError::RuntimeError(format!("Failed to write to memory: {:?}", e)))?;
        
        // Get the main function
        let func: TypedFunction<(u32, u32), u32> = instance
            .exports
            .get_typed_function(&self.store, function_name)
            .map_err(|e| ExecutionError::RuntimeError(format!("Function '{}' not found: {}", function_name, e)))?;
        
        // Call the function
        let result_ptr = func.call(&mut self.store, args_ptr, args_len)
            .map_err(|e| ExecutionError::RuntimeError(format!("Function execution failed: {}", e)))?;
        
        // Read the result
        self.read_region(memory, result_ptr)
    }

    fn execute_simple_function(
        &mut self,
        instance: &Instance,
        function_name: &str,
    ) -> Result<Vec<u8>, ExecutionError> {
        // Try no-arg function
        if let Ok(func) = instance.exports.get_typed_function::<(), ()>(&self.store, function_name) {
            func.call(&mut self.store)
                .map_err(|e| ExecutionError::RuntimeError(format!("Function execution failed: {}", e)))?;
            return Ok(vec![]);
        }
        
        Err(ExecutionError::RuntimeError(format!("Could not execute function '{}'", function_name)))
    }

    fn read_region(&self, memory: &Memory, ptr: u32) -> Result<Vec<u8>, ExecutionError> {
        let view = memory.view(&self.store);
        
        // Read the CosmWasm Region struct (offset, capacity, length)
        let mut region_data = vec![0u8; 12];
        view.read(ptr as u64, &mut region_data)
            .map_err(|e| ExecutionError::RuntimeError(format!("Failed to read region: {:?}", e)))?;
        
        let offset = u32::from_le_bytes([region_data[0], region_data[1], region_data[2], region_data[3]]);
        let _capacity = u32::from_le_bytes([region_data[4], region_data[5], region_data[6], region_data[7]]);
        let length = u32::from_le_bytes([region_data[8], region_data[9], region_data[10], region_data[11]]);
        
        // Read the actual data
        let mut buffer = vec![0u8; length as usize];
        view.read(offset as u64, &mut buffer)
            .map_err(|e| ExecutionError::RuntimeError(format!("Failed to read data: {:?}", e)))?;
        
        Ok(buffer)
    }

    fn create_imports(&mut self) -> Result<wasmer::Imports, ExecutionError> {
        let _state_changes = self.state_changes.clone();
        let _events = self.events.clone();
        let _storage_prefix = self.storage_prefix.clone();
        
        // Create empty imports for now - we'll implement host functions properly later
        let imports = imports! {};
        
        Ok(imports)
    }

    pub fn get_state_changes(&self) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
        self.state_changes.lock().unwrap().clone()
    }

    pub fn get_events(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.events.lock().unwrap().clone()
    }
}