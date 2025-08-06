/// IBC Transfer Module Contract
/// 
/// This contract handles all IBC token transfer operations including:
/// - Cross-chain token transfers
/// - Voucher creation and redemption
/// - Port binding for transfer protocol
/// - Timeout and acknowledgment handling

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use crate::modules::ibc::transfer::{TransferModule, FungibleTokenPacketData, DenomTrace};
use crate::modules::ibc::channel::{ChannelModule, Height, Packet};
use crate::modules::bank::BankModule;
use crate::Balance;

/// IBC Transfer contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct IbcTransferContract {
    /// The underlying transfer module
    transfer_module: TransferModule,
    /// Channel module for IBC operations
    channel_module: ChannelModule,
    /// Bank module for token operations
    bank_module: BankModule,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
}

/// Response from transfer operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct TransferOperationResponse {
    pub success: bool,
    pub packet_data: Option<String>,
    pub voucher_denom: Option<String>,
    pub amount: Option<Balance>,
    pub events: Vec<String>,
    pub error: Option<String>,
}

/// Cross-chain transfer request
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct TransferRequest {
    pub sender: String,
    pub receiver: String, // Receiving address on destination chain
    pub token: String,
    pub amount: Balance,
    pub source_port: String,
    pub source_channel: String,
    pub timeout_height: Option<u64>,
    pub timeout_timestamp: Option<u64>,
    pub memo: Option<String>,
}

/// Voucher token information
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct VoucherInfo {
    pub denom: String,
    pub trace: String,
    pub is_native: bool,
    pub total_supply: Balance,
}

#[near_bindgen]
impl IbcTransferContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: Option<AccountId>) -> Self {
        Self {
            transfer_module: TransferModule::new(),
            channel_module: ChannelModule::new(),
            bank_module: BankModule::new(),
            router_contract,
            owner,
        }
    }

    // =============================================================================
    // Transfer Functions
    // =============================================================================

    /// Initiate a cross-chain token transfer
    pub fn send_transfer(&mut self, request: TransferRequest) -> TransferOperationResponse {
        self.assert_authorized_caller();
        
        // Validate sender authorization
        let caller = env::predecessor_account_id();
        if caller != request.sender && !self.is_router_or_owner(&caller) {
            return TransferOperationResponse {
                success: false,
                packet_data: None,
                voucher_denom: None,
                amount: Some(request.amount),
                events: vec![],
                error: Some("Unauthorized: caller cannot transfer from this account".to_string()),
            };
        }

        let height = Height {
            revision_number: 0,
            revision_height: request.timeout_height.unwrap_or(0),
        };
        
        match self.transfer_module.send_transfer(
            &mut self.channel_module,
            &mut self.bank_module,
            request.source_port,
            request.source_channel,
            request.token.clone(),
            request.amount,
            request.sender.to_string(),
            request.receiver.clone(),
            height,
            request.timeout_timestamp.unwrap_or(0),
            request.memo.clone(),
        ) {
            Ok(packet_sequence) => {
                env::log_str(&format!(
                    "Transfer initiated: {} {} from {} to destination (seq {})", 
                    request.amount, 
                    request.token, 
                    request.sender,
                    packet_sequence
                ));
                
                // Create packet data for response
                let packet_data = FungibleTokenPacketData {
                    denom: request.token.clone(),
                    amount: request.amount.to_string(),
                    sender: request.sender.to_string(),
                    receiver: request.receiver.clone(),
                    memo: request.memo.clone().unwrap_or_default(),
                };
                
                TransferOperationResponse {
                    success: true,
                    packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                    voucher_denom: None,
                    amount: Some(request.amount),
                    events: vec!["send_transfer".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Transfer failed: {:?}", e));
                TransferOperationResponse {
                    success: false,
                    packet_data: None,
                    voucher_denom: None,
                    amount: Some(request.amount),
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Receive a transfer packet (called when packet arrives from another chain)
    pub fn receive_transfer(
        &mut self, 
        packet_data: Base64VecU8,
        sequence: u64,
        source_port: String,
        source_channel: String,
        destination_port: String,
        destination_channel: String,
        timeout_height: u64,
        timeout_timestamp: u64,
    ) -> TransferOperationResponse {
        self.assert_authorized_caller();
        
        // Create packet struct
        let packet = Packet {
            sequence,
            source_port,
            source_channel,
            destination_port,
            destination_channel,
            data: packet_data.clone().into(),
            timeout_height: Height {
                revision_number: 0,
                revision_height: timeout_height,
            },
            timeout_timestamp,
        };
        
        // Process the transfer
        match self.transfer_module.receive_transfer(
            &self.channel_module,
            &mut self.bank_module,
            &packet,
        ) {
            Ok(_ack) => {
                // Decode transfer data to get details for logging
                if let Ok(data) = serde_json::from_slice::<FungibleTokenPacketData>(&packet.data) {
                    env::log_str(&format!(
                        "Transfer received: {} {} to {}", 
                        data.amount, 
                        data.denom, 
                        data.receiver
                    ));
                    
                    TransferOperationResponse {
                        success: true,
                        packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                        voucher_denom: None, // We don't get voucher denom from ack
                        amount: Some(data.amount.parse().unwrap_or(0)),
                        events: vec!["receive_transfer".to_string()],
                        error: None,
                    }
                } else {
                    TransferOperationResponse {
                        success: true,
                        packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                        voucher_denom: None,
                        amount: None,
                        events: vec!["receive_transfer".to_string()],
                        error: None,
                    }
                }
            }
            Err(e) => {
                env::log_str(&format!("Transfer receive failed: {:?}", e));
                TransferOperationResponse {
                    success: false,
                    packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                    voucher_denom: None,
                    amount: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Handle transfer acknowledgment (when transfer succeeds on destination)
    pub fn acknowledge_transfer(&mut self, packet_data: Base64VecU8, ack: Base64VecU8) -> TransferOperationResponse {
        self.assert_authorized_caller();
        
        let transfer_data: Result<FungibleTokenPacketData, _> = serde_json::from_slice(&packet_data.0);
        
        match transfer_data {
            Ok(data) => {
                // Check if acknowledgment indicates success or failure
                let ack_str = String::from_utf8(ack.clone().into()).unwrap_or_default();
                let is_success = !ack_str.contains("error") && !ack_str.contains("failed");
                
                if is_success {
                    env::log_str(&format!("Transfer acknowledged successfully: {} {}", data.amount, data.denom));
                    TransferOperationResponse {
                        success: true,
                        packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                        voucher_denom: None,
                        amount: Some(data.amount.parse().unwrap_or(0)),
                        events: vec!["acknowledge_transfer".to_string()],
                        error: None,
                    }
                } else {
                    // Transfer failed, need to refund tokens
                    match self.transfer_module.refund_tokens(data.clone()) {
                        Ok(_) => {
                            env::log_str(&format!("Transfer failed, tokens refunded: {} {}", data.amount, data.denom));
                            TransferOperationResponse {
                                success: false,
                                packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                                voucher_denom: None,
                                amount: Some(data.amount.parse().unwrap_or(0)),
                                events: vec!["transfer_failed_refunded".to_string()],
                                error: Some("Transfer failed on destination".to_string()),
                            }
                        }
                        Err(e) => {
                            env::log_str(&format!("Transfer failed and refund failed: {:?}", e));
                            TransferOperationResponse {
                                success: false,
                                packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                                voucher_denom: None,
                                amount: Some(data.amount.parse().unwrap_or(0)),
                                events: vec!["transfer_failed".to_string()],
                                error: Some(format!("Transfer and refund failed: {:?}", e)),
                            }
                        }
                    }
                }
            }
            Err(e) => {
                env::log_str(&format!("Invalid transfer packet data in ack: {:?}", e));
                TransferOperationResponse {
                    success: false,
                    packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                    voucher_denom: None,
                    amount: None,
                    events: vec![],
                    error: Some("Invalid packet data format".to_string()),
                }
            }
        }
    }

    /// Handle transfer timeout (when transfer expires)
    pub fn timeout_transfer(&mut self, packet_data: Base64VecU8) -> TransferOperationResponse {
        self.assert_authorized_caller();
        
        let transfer_data: Result<FungibleTokenPacketData, _> = serde_json::from_slice(&packet_data.0);
        
        match transfer_data {
            Ok(data) => {
                // Refund tokens to sender
                match self.transfer_module.refund_tokens(data.clone()) {
                    Ok(_) => {
                        env::log_str(&format!("Transfer timed out, tokens refunded: {} {}", data.amount, data.denom));
                        TransferOperationResponse {
                            success: true,
                            packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                            voucher_denom: None,
                            amount: Some(data.amount.parse().unwrap_or(0)),
                            events: vec!["timeout_transfer".to_string()],
                            error: None,
                        }
                    }
                    Err(e) => {
                        env::log_str(&format!("Transfer timeout refund failed: {:?}", e));
                        TransferOperationResponse {
                            success: false,
                            packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                            voucher_denom: None,
                            amount: Some(data.amount.parse().unwrap_or(0)),
                            events: vec![],
                            error: Some(format!("Timeout refund failed: {:?}", e)),
                        }
                    }
                }
            }
            Err(e) => {
                env::log_str(&format!("Invalid transfer packet data in timeout: {:?}", e));
                TransferOperationResponse {
                    success: false,
                    packet_data: Some(serde_json::to_string(&packet_data).unwrap_or_default()),
                    voucher_denom: None,
                    amount: None,
                    events: vec![],
                    error: Some("Invalid packet data format".to_string()),
                }
            }
        }
    }

    // =============================================================================
    // Query Functions
    // =============================================================================

    /// Get denomination trace information
    pub fn get_denom_trace(&self, hash: String) -> Option<DenomTrace> {
        self.assert_authorized_caller();
        self.transfer_module.get_denom_trace(&hash)
    }

    /// Get all denomination traces
    pub fn get_all_denom_traces(&self) -> Vec<DenomTrace> {
        self.assert_authorized_caller();
        self.transfer_module.get_all_denom_traces()
    }

    /// Get voucher balance for an account
    pub fn get_voucher_balance(&self, account: AccountId, denom: String) -> Balance {
        self.assert_authorized_caller();
        self.transfer_module.get_voucher_balance(account.to_string(), denom)
    }

    /// Get all voucher balances for an account
    pub fn get_all_voucher_balances(&self, account: AccountId) -> Vec<(String, Balance)> {
        self.assert_authorized_caller();
        self.transfer_module.get_all_voucher_balances(account.to_string())
    }

    /// Check if a denomination is a voucher (originated from another chain)
    pub fn is_voucher_denom(&self, denom: String) -> bool {
        self.assert_authorized_caller();
        self.transfer_module.is_voucher_denom(&denom)
    }

    /// Get the original denomination from a voucher
    pub fn get_original_denom(&self, voucher_denom: String) -> Option<String> {
        self.assert_authorized_caller();
        self.transfer_module.get_original_denom(voucher_denom)
    }

    /// Get voucher information
    pub fn get_voucher_info(&self, denom: String) -> Option<VoucherInfo> {
        self.assert_authorized_caller();
        
        if let Some(trace) = self.transfer_module.get_denom_trace(&denom) {
            Some(VoucherInfo {
                denom: denom.clone(),
                trace: trace.path,
                is_native: false,
                total_supply: self.transfer_module.get_total_voucher_supply(denom),
            })
        } else {
            None
        }
    }

    // =============================================================================
    // Port Management
    // =============================================================================

    /// Bind the transfer port
    pub fn bind_port(&mut self, port_id: String) -> bool {
        self.assert_authorized_caller();
        
        self.transfer_module.bind_port(port_id.clone());
        env::log_str(&format!("Transfer port bound: {}", port_id));
        true
    }

    /// Check if port is bound
    pub fn is_port_bound(&self, port_id: String) -> bool {
        self.assert_authorized_caller();
        self.transfer_module.is_port_bound(port_id)
    }

    // =============================================================================
    // Admin and Configuration Functions
    // =============================================================================

    /// Update the router contract address
    pub fn update_router_contract(&mut self, new_router: AccountId) {
        self.assert_owner();
        self.router_contract = Some(new_router.clone());
        env::log_str(&format!("Updated router contract to: {}", new_router));
    }

    /// Get current router contract
    pub fn get_router_contract(&self) -> Option<AccountId> {
        self.router_contract.clone()
    }

    /// Health check for the transfer module
    pub fn health_check(&self) -> bool {
        // Check if the transfer module is functioning
        true // In a full implementation, would perform actual health checks
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "module_type": "ibc_transfer",
            "version": "1.0.0",
            "description": "IBC Transfer Module",
            "functions": [
                "send_transfer",
                "receive_transfer",
                "acknowledge_transfer",
                "timeout_transfer",
                "get_denom_trace",
                "get_all_denom_traces",
                "get_voucher_balance",
                "get_all_voucher_balances",
                "is_voucher_denom",
                "get_original_denom",
                "get_voucher_info",
                "bind_port",
                "is_port_bound"
            ]
        })
    }

    // =============================================================================
    // Helper Functions
    // =============================================================================

    /// Check if caller is router or owner
    fn is_router_or_owner(&self, caller: &AccountId) -> bool {
        caller == &self.owner || 
        self.router_contract.as_ref().map_or(false, |router| caller == router)
    }

    /// Assert that the caller is authorized (owner or router)
    fn assert_authorized_caller(&self) {
        let caller = env::predecessor_account_id();
        
        let is_owner = caller == self.owner;
        let is_router = self.router_contract.as_ref().map_or(false, |router| caller == *router);
        
        assert!(
            is_owner || is_router,
            "Unauthorized: only owner or router can call this function"
        );
    }

    /// Assert that the caller is the contract owner
    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can perform this action"
        );
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.assert_owner();
        let old_owner = self.owner.clone();
        self.owner = new_owner.clone();
        env::log_str(&format!("Ownership transferred from {} to {}", old_owner, new_owner));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, VMContext};

    fn get_context(predecessor: AccountId) -> VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .build()
    }

    #[test]
    fn test_transfer_contract_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcTransferContract::new(accounts(1), Some(accounts(2)));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, Some(accounts(2)));
    }

    #[test]
    fn test_port_binding() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut contract = IbcTransferContract::new(accounts(1), None);
        
        let success = contract.bind_port("transfer".to_string());
        assert!(success);
        
        let is_bound = contract.is_port_bound("transfer".to_string());
        assert!(is_bound);
    }

    #[test]
    fn test_health_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcTransferContract::new(accounts(1), None);
        assert!(contract.health_check());
    }

    #[test]
    fn test_authorization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcTransferContract::new(accounts(1), Some(accounts(2)));
        
        // Owner should be authorized
        contract.assert_authorized_caller();
        
        // Test router access
        let router_context = get_context(accounts(2));
        testing_env!(router_context);
        contract.assert_authorized_caller();
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_caller() {
        let context = get_context(accounts(3)); // Unauthorized account
        testing_env!(context);
        
        let contract = IbcTransferContract::new(accounts(1), Some(accounts(2)));
        contract.assert_authorized_caller();
    }
}