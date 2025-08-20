/// CosmWasm Relayer Service
/// 
/// This service monitors NEAR contracts for CosmWasm execution requests,
/// executes them using Wasmer, and submits results back to NEAR.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time;
use near_jsonrpc_client::{JsonRpcClient, methods};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::types::{BlockReference, FunctionArgs};
use near_primitives::views::QueryRequest;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, debug};

use super::{WasmerExecutionService, StateManager};
use super::types::{ExecutionResult, CosmWasmEnv};

/// Configuration for the CosmWasm relayer service
#[derive(Debug, Clone, Deserialize)]
pub struct CosmWasmRelayerConfig {
    /// NEAR RPC endpoint
    pub near_rpc_url: String,
    
    /// NEAR account that will submit results
    pub relayer_account_id: String,
    
    /// Private key for the relayer account
    pub relayer_private_key: String,
    
    /// Contract to monitor for CosmWasm requests
    pub wasm_module_contract: String,
    
    /// Polling interval in milliseconds
    pub polling_interval_ms: u64,
    
    /// Maximum retries for failed submissions
    pub max_retries: u32,
    
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

/// Execution request from NEAR contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub request_id: String,
    pub contract_address: String,
    pub code_id: u64,
    pub entry_point: String,
    pub msg: Vec<u8>,
    pub sender: String,
    pub funds: Vec<Coin>,
    pub block_height: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

/// Status of an execution request
#[derive(Debug, Clone, PartialEq)]
pub enum RequestStatus {
    Pending,
    Executing,
    Executed,
    Submitted,
    Failed(String),
}

/// Tracked execution request
struct TrackedRequest {
    request: ExecutionRequest,
    status: RequestStatus,
    retries: u32,
    result: Option<ExecutionResult>,
}

/// CosmWasm Relayer Service
pub struct CosmWasmRelayerService {
    config: CosmWasmRelayerConfig,
    near_client: JsonRpcClient,
    execution_service: Arc<WasmerExecutionService>,
    pending_requests: Arc<RwLock<Vec<TrackedRequest>>>,
    request_sender: mpsc::Sender<ExecutionRequest>,
    request_receiver: mpsc::Receiver<ExecutionRequest>,
}

impl CosmWasmRelayerService {
    /// Create a new CosmWasm relayer service
    pub fn new(config: CosmWasmRelayerConfig) -> Self {
        let near_client = JsonRpcClient::connect(&config.near_rpc_url);
        let state_manager = Arc::new(StateManager::new());
        let execution_service = Arc::new(WasmerExecutionService::new(state_manager));
        let (tx, rx) = mpsc::channel(100);
        
        Self {
            config,
            near_client,
            execution_service,
            pending_requests: Arc::new(RwLock::new(Vec::new())),
            request_sender: tx,
            request_receiver: rx,
        }
    }
    
    /// Start the relayer service
    pub async fn start(mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting CosmWasm relayer service");
        info!("Monitoring contract: {}", self.config.wasm_module_contract);
        info!("Relayer account: {}", self.config.relayer_account_id);
        
        // Start monitoring task
        let monitor_handle = self.spawn_monitor_task();
        
        // Start execution worker
        let execution_handle = self.spawn_execution_worker();
        
        // Start submission worker
        let submission_handle = self.spawn_submission_worker();
        
        // Wait for tasks
        tokio::select! {
            result = monitor_handle => {
                error!("Monitor task ended: {:?}", result);
            }
            result = execution_handle => {
                error!("Execution worker ended: {:?}", result);
            }
            result = submission_handle => {
                error!("Submission worker ended: {:?}", result);
            }
        }
        
        Ok(())
    }
    
    /// Spawn the monitoring task
    fn spawn_monitor_task(&self) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let near_client = self.near_client.clone();
        let sender = self.request_sender.clone();
        let pending = self.pending_requests.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(config.polling_interval_ms));
            let mut last_processed_height = 0u64;
            
            loop {
                interval.tick().await;
                
                match Self::poll_for_requests(&near_client, &config.wasm_module_contract, last_processed_height).await {
                    Ok(requests) => {
                        for request in requests {
                            if request.block_height > last_processed_height {
                                last_processed_height = request.block_height;
                            }
                            
                            // Check if already processing
                            let mut pending_list = pending.write().await;
                            let already_exists = pending_list.iter().any(|t| t.request.request_id == request.request_id);
                            
                            if !already_exists {
                                info!("📦 New execution request: {}", request.request_id);
                                pending_list.push(TrackedRequest {
                                    request: request.clone(),
                                    status: RequestStatus::Pending,
                                    retries: 0,
                                    result: None,
                                });
                                
                                if let Err(e) = sender.send(request).await {
                                    error!("Failed to queue request: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to poll for requests: {}", e);
                    }
                }
            }
        })
    }
    
    /// Spawn the execution worker
    fn spawn_execution_worker(&mut self) -> tokio::task::JoinHandle<()> {
        let execution_service = self.execution_service.clone();
        let pending = self.pending_requests.clone();
        let mut receiver = self.request_receiver.clone();
        let config = self.config.clone();
        let near_client = self.near_client.clone();
        
        tokio::spawn(async move {
            while let Some(request) = receiver.recv().await {
                info!("⚙️ Executing request: {}", request.request_id);
                
                // Update status
                {
                    let mut pending_list = pending.write().await;
                    if let Some(tracked) = pending_list.iter_mut().find(|t| t.request.request_id == request.request_id) {
                        tracked.status = RequestStatus::Executing;
                    }
                }
                
                // Get WASM code from NEAR
                match Self::fetch_wasm_code(&near_client, &config.wasm_module_contract, request.code_id).await {
                    Ok(wasm_code) => {
                        // Create CosmWasm environment
                        let env = Self::create_cosmwasm_env(&request);
                        
                        // Execute the contract
                        match execution_service.execute_contract(
                            &request.contract_address,
                            &wasm_code,
                            &request.entry_point,
                            &request.msg,
                            env,
                        ).await {
                            Ok(result) => {
                                info!("✅ Execution successful for: {}", request.request_id);
                                
                                let mut pending_list = pending.write().await;
                                if let Some(tracked) = pending_list.iter_mut().find(|t| t.request.request_id == request.request_id) {
                                    tracked.status = RequestStatus::Executed;
                                    tracked.result = Some(result);
                                }
                            }
                            Err(e) => {
                                error!("❌ Execution failed for {}: {}", request.request_id, e);
                                
                                let mut pending_list = pending.write().await;
                                if let Some(tracked) = pending_list.iter_mut().find(|t| t.request.request_id == request.request_id) {
                                    tracked.status = RequestStatus::Failed(format!("Execution error: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch WASM code for {}: {}", request.request_id, e);
                        
                        let mut pending_list = pending.write().await;
                        if let Some(tracked) = pending_list.iter_mut().find(|t| t.request.request_id == request.request_id) {
                            tracked.status = RequestStatus::Failed(format!("Failed to fetch code: {}", e));
                        }
                    }
                }
            }
        })
    }
    
    /// Spawn the submission worker
    fn spawn_submission_worker(&self) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let near_client = self.near_client.clone();
        let pending = self.pending_requests.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(config.retry_delay_ms));
            
            loop {
                interval.tick().await;
                
                // Find executed requests that need submission
                let to_submit: Vec<TrackedRequest> = {
                    let pending_list = pending.read().await;
                    pending_list.iter()
                        .filter(|t| t.status == RequestStatus::Executed && t.result.is_some())
                        .cloned()
                        .collect()
                };
                
                for mut tracked in to_submit {
                    info!("📤 Submitting result for: {}", tracked.request.request_id);
                    
                    if let Some(result) = &tracked.result {
                        match Self::submit_result(
                            &near_client,
                            &config,
                            &tracked.request,
                            result,
                        ).await {
                            Ok(_) => {
                                info!("✅ Result submitted for: {}", tracked.request.request_id);
                                
                                // Update status and remove from pending
                                let mut pending_list = pending.write().await;
                                if let Some(pos) = pending_list.iter().position(|t| t.request.request_id == tracked.request.request_id) {
                                    pending_list.remove(pos);
                                }
                            }
                            Err(e) => {
                                error!("❌ Failed to submit result for {}: {}", tracked.request.request_id, e);
                                tracked.retries += 1;
                                
                                if tracked.retries >= config.max_retries {
                                    error!("Max retries reached for: {}", tracked.request.request_id);
                                    
                                    let mut pending_list = pending.write().await;
                                    if let Some(tracked) = pending_list.iter_mut().find(|t| t.request.request_id == tracked.request.request_id) {
                                        tracked.status = RequestStatus::Failed(format!("Max retries reached: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    }
    
    /// Poll NEAR contract for execution requests
    async fn poll_for_requests(
        client: &JsonRpcClient,
        contract_id: &str,
        last_height: u64,
    ) -> Result<Vec<ExecutionRequest>, Box<dyn std::error::Error>> {
        // This is a placeholder - in reality, we'd query contract events or state
        // For now, we'll query a hypothetical view method
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::CallFunction {
                account_id: contract_id.parse()?,
                method_name: "get_pending_executions".to_string(),
                args: FunctionArgs::from(format!(r#"{{"after_height": {}}}"#, last_height).into_bytes()),
            },
        };
        
        let response = client.call(request).await?;
        
        if let QueryResponseKind::CallResult(result) = response.kind {
            let requests: Vec<ExecutionRequest> = serde_json::from_slice(&result.result)?;
            Ok(requests)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Fetch WASM code from NEAR contract
    async fn fetch_wasm_code(
        client: &JsonRpcClient,
        contract_id: &str,
        code_id: u64,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::latest(),
            request: QueryRequest::CallFunction {
                account_id: contract_id.parse()?,
                method_name: "get_code".to_string(),
                args: FunctionArgs::from(format!(r#"{{"code_id": {}}}"#, code_id).into_bytes()),
            },
        };
        
        let response = client.call(request).await?;
        
        if let QueryResponseKind::CallResult(result) = response.kind {
            Ok(result.result)
        } else {
            Err("Failed to fetch WASM code".into())
        }
    }
    
    /// Create CosmWasm environment from request
    fn create_cosmwasm_env(request: &ExecutionRequest) -> CosmWasmEnv {
        CosmWasmEnv {
            block_height: request.block_height,
            block_time: request.timestamp,
            chain_id: "near-cosmwasm".to_string(),
            contract_address: request.contract_address.clone(),
            sender: request.sender.clone(),
        }
    }
    
    /// Submit execution result back to NEAR
    async fn submit_result(
        client: &JsonRpcClient,
        config: &CosmWasmRelayerConfig,
        request: &ExecutionRequest,
        result: &ExecutionResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create the execution result input
        let result_input = serde_json::json!({
            "contract_addr": request.contract_address,
            "execution_result": {
                "data": result.data.as_ref().map(|d| base64::encode(d)),
                "state_changes": result.state_changes.iter().map(|sc| {
                    serde_json::json!({
                        "key": base64::encode(&sc.key),
                        "value": sc.value.as_ref().map(|v| base64::encode(v)),
                        "operation": if sc.value.is_some() { "Set" } else { "Remove" }
                    })
                }).collect::<Vec<_>>(),
                "events": result.events.iter().map(|e| {
                    serde_json::json!({
                        "event_type": e.r#type,
                        "attributes": e.attributes.iter().map(|a| [&a.key, &a.value]).collect::<Vec<_>>()
                    })
                }).collect::<Vec<_>>(),
                "gas_used": result.gas_used,
            }
        });
        
        // TODO: Sign and send transaction to NEAR
        // This requires proper key management and transaction signing
        debug!("Would submit result: {:?}", result_input);
        
        Ok(())
    }
}

// Re-export for convenience
pub use self::CosmWasmRelayerConfig as Config;
pub use self::CosmWasmRelayerService as Service;