/// IBC Channel Module Contract
/// 
/// This contract handles all IBC channel operations including:
/// - Channel handshake (open, try, ack, confirm)
/// - Packet lifecycle (send, receive, acknowledge, timeout)
/// - Channel and packet queries
/// - Port binding and management

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use base64::{Engine as _, engine::general_purpose};

use crate::modules::ibc::channel::{ChannelModule, ChannelEnd, Packet, Acknowledgement};
use crate::modules::ibc::channel::types::{Height};

/// IBC Channel contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct IbcChannelContract {
    /// The underlying channel module
    channel_module: ChannelModule,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
}

/// Response from channel operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ChannelOperationResponse {
    pub success: bool,
    pub channel_id: Option<String>,
    pub port_id: Option<String>,
    pub data: Option<String>,
    pub events: Vec<String>,
    pub error: Option<String>,
}

/// Packet data for send/receive operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct PacketData {
    pub sequence: u64,
    pub source_port: String,
    pub source_channel: String,
    pub destination_port: String,
    pub destination_channel: String,
    pub data: String,
    pub timeout_height: Option<u64>,
    pub timeout_timestamp: Option<u64>,
}

impl From<PacketData> for Packet {
    fn from(data: PacketData) -> Self {
        Packet {
            sequence: data.sequence,
            source_port: data.source_port,
            source_channel: data.source_channel,
            destination_port: data.destination_port,
            destination_channel: data.destination_channel,
            data: data.data.into(),
            timeout_height: Height {
                revision_number: 0,
                revision_height: data.timeout_height.unwrap_or(0),
            },
            timeout_timestamp: data.timeout_timestamp.unwrap_or(0),
        }
    }
}

impl From<Packet> for PacketData {
    fn from(packet: Packet) -> Self {
        PacketData {
            sequence: packet.sequence,
            source_port: packet.source_port,
            source_channel: packet.source_channel,
            destination_port: packet.destination_port,
            destination_channel: packet.destination_channel,
            data: general_purpose::STANDARD.encode(&packet.data),
            timeout_height: Some(packet.timeout_height.revision_height),
            timeout_timestamp: Some(packet.timeout_timestamp),
        }
    }
}

#[near_bindgen]
impl IbcChannelContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: Option<AccountId>) -> Self {
        Self {
            channel_module: ChannelModule::new(),
            router_contract,
            owner,
        }
    }

    // =============================================================================
    // Channel Handshake Functions
    // =============================================================================

    /// Open a new IBC channel (first step of handshake)
    pub fn channel_open_init(
        &mut self,
        port_id: String,
        channel: ChannelEnd,
    ) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        match self.channel_module.channel_open_init(port_id.clone(), channel) {
            Ok(channel_id) => {
                env::log_str(&format!("Channel opened: {}/{}", port_id, channel_id));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(channel_id),
                    port_id: Some(port_id),
                    data: None,
                    events: vec!["channel_open_init".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Channel open failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: None,
                    port_id: Some(port_id),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Try to open a channel (second step of handshake)
    pub fn channel_open_try(
        &mut self,
        port_id: String,
        channel: ChannelEnd,
        counterparty_version: String,
        proof_init: Base64VecU8,
        proof_height: u64,
    ) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        let connection_hops = channel.connection_hops.clone();
        match self.channel_module.channel_open_try(
            port_id.clone(),
            channel,
            counterparty_version,
            connection_hops,
            serde_json::from_slice(&proof_init.0).unwrap_or_default(),
            proof_height,
        ) {
            Ok(channel_id) => {
                env::log_str(&format!("Channel try successful: {}/{}", port_id, channel_id));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(channel_id),
                    port_id: Some(port_id),
                    data: None,
                    events: vec!["channel_open_try".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Channel try failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: None,
                    port_id: Some(port_id),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Acknowledge channel opening (third step of handshake)
    pub fn channel_open_ack(
        &mut self,
        port_id: String,
        channel_id: String,
        _counterparty_channel_id: String,
        counterparty_version: String,
        proof_try: Base64VecU8,
        proof_height: u64,
    ) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        match self.channel_module.channel_open_ack(
            port_id.clone(),
            channel_id.clone(),
            counterparty_version,
            proof_try.into(),
            proof_height,
        ) {
            Ok(_) => {
                env::log_str(&format!("Channel ack successful: {}/{}", port_id, channel_id));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(channel_id),
                    port_id: Some(port_id),
                    data: None,
                    events: vec!["channel_open_ack".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Channel ack failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: Some(channel_id),
                    port_id: Some(port_id),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Confirm channel opening (final step of handshake)
    pub fn channel_open_confirm(
        &mut self,
        port_id: String,
        channel_id: String,
        proof_ack: Base64VecU8,
        proof_height: u64,
    ) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        match self.channel_module.channel_open_confirm(
            port_id.clone(),
            channel_id.clone(),
            proof_ack.into(),
            proof_height,
        ) {
            Ok(_) => {
                env::log_str(&format!("Channel confirmed: {}/{}", port_id, channel_id));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(channel_id),
                    port_id: Some(port_id),
                    data: None,
                    events: vec!["channel_open_confirm".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Channel confirm failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: Some(channel_id),
                    port_id: Some(port_id),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    // =============================================================================
    // Packet Lifecycle Functions
    // =============================================================================

    /// Send a packet through the channel
    pub fn send_packet(&mut self, packet_data: PacketData) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        let packet: Packet = packet_data.into();
        
        match self.channel_module.send_packet(
            packet.source_port.clone(),
            packet.source_channel.clone(), 
            packet.timeout_height.clone(),
            packet.timeout_timestamp,
            packet.data.clone(),
        ) {
            Ok(_) => {
                env::log_str(&format!(
                    "Packet sent: {}/{} seq={}", 
                    packet.source_port, 
                    packet.source_channel, 
                    packet.sequence
                ));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(packet.source_channel),
                    port_id: Some(packet.source_port),
                    data: Some(general_purpose::STANDARD.encode(&packet.data)),
                    events: vec!["send_packet".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Packet send failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: Some(packet.source_channel),
                    port_id: Some(packet.source_port),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Receive a packet (for the destination chain)
    pub fn receive_packet(
        &mut self,
        packet_data: PacketData,
        proof_commitment: Base64VecU8,
        proof_height: u64,
    ) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        let packet: Packet = packet_data.into();
        
        match self.channel_module.recv_packet(
            packet.clone(),
            proof_commitment.into(),
            proof_height,
        ) {
            Ok(_acknowledgement) => {
                env::log_str(&format!(
                    "Packet received: {}/{} seq={}", 
                    packet.destination_port, 
                    packet.destination_channel, 
                    packet.sequence
                ));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(packet.destination_channel),
                    port_id: Some(packet.destination_port),
                    data: Some("success".to_string()),
                    events: vec!["recv_packet".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Packet receive failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: Some(packet.destination_channel),
                    port_id: Some(packet.destination_port),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Acknowledge a packet (for the source chain)
    pub fn acknowledge_packet(
        &mut self,
        packet_data: PacketData,
        acknowledgement: Base64VecU8,
        proof_acked: Base64VecU8,
        proof_height: u64,
    ) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        let packet: Packet = packet_data.into();
        
        match self.channel_module.acknowledge_packet(
            packet.clone(),
            Acknowledgement { data: acknowledgement.into() },
            proof_acked.into(),
            proof_height,
        ) {
            Ok(_) => {
                env::log_str(&format!(
                    "Packet acknowledged: {}/{} seq={}", 
                    packet.source_port, 
                    packet.source_channel, 
                    packet.sequence
                ));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(packet.source_channel),
                    port_id: Some(packet.source_port),
                    data: None,
                    events: vec!["acknowledge_packet".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Packet acknowledgment failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: Some(packet.source_channel),
                    port_id: Some(packet.source_port),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Timeout a packet (when it expires)
    pub fn timeout_packet(
        &mut self,
        packet_data: PacketData,
        proof_unreceived: Base64VecU8,
        proof_height: u64,
    ) -> ChannelOperationResponse {
        self.assert_authorized_caller();
        
        let packet: Packet = packet_data.into();
        
        match self.channel_module.timeout_packet(
            packet.clone(),
            proof_unreceived.into(),
            proof_height,
            0, // next_sequence_recv - should be passed as parameter
        ) {
            Ok(_) => {
                env::log_str(&format!(
                    "Packet timed out: {}/{} seq={}", 
                    packet.source_port, 
                    packet.source_channel, 
                    packet.sequence
                ));
                ChannelOperationResponse {
                    success: true,
                    channel_id: Some(packet.source_channel),
                    port_id: Some(packet.source_port),
                    data: None,
                    events: vec!["timeout_packet".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Packet timeout failed: {:?}", e));
                ChannelOperationResponse {
                    success: false,
                    channel_id: Some(packet.source_channel),
                    port_id: Some(packet.source_port),
                    data: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    // =============================================================================
    // Query Functions
    // =============================================================================

    /// Get channel information
    pub fn get_channel(&self, port_id: String, channel_id: String) -> Option<ChannelEnd> {
        self.assert_authorized_caller();
        self.channel_module.get_channel(port_id, channel_id)
    }

    /// Get packet commitment
    pub fn query_packet_commitment(
        &self,
        port_id: String,
        channel_id: String,
        sequence: u64,
    ) -> Option<String> {
        self.assert_authorized_caller();
        
        self.channel_module
            .get_packet_commitment(&port_id, &channel_id, sequence)
            .map(|commitment| hex::encode(&commitment.data))
    }

    /// Get packet acknowledgment
    pub fn query_packet_acknowledgment(
        &self,
        port_id: String,
        channel_id: String,
        sequence: u64,
    ) -> Option<String> {
        self.assert_authorized_caller();
        
        self.channel_module
            .get_packet_acknowledgement(&port_id, &channel_id, sequence)
            .map(|ack| hex::encode(&ack.data))
    }

    /// Get packet receipt (for unordered channels)
    pub fn query_packet_receipt(
        &self,
        port_id: String,
        channel_id: String,
        sequence: u64,
    ) -> bool {
        self.assert_authorized_caller();
        self.channel_module.get_packet_receipt(&port_id, &channel_id, sequence).is_some()
    }

    /// Get next sequence receive
    pub fn query_next_sequence_recv(
        &self,
        port_id: String,
        channel_id: String,
    ) -> u64 {
        self.assert_authorized_caller();
        self.channel_module.get_next_sequence_recv(&port_id, &channel_id)
    }

    /// Get next sequence send
    pub fn query_next_sequence_send(
        &self,
        port_id: String,
        channel_id: String,
    ) -> u64 {
        self.assert_authorized_caller();
        self.channel_module.get_next_sequence_send(&port_id, &channel_id)
    }

    /// Get next sequence ack
    pub fn query_next_sequence_ack(
        &self,
        port_id: String,
        channel_id: String,
    ) -> u64 {
        self.assert_authorized_caller();
        self.channel_module.get_next_sequence_ack(&port_id, &channel_id)
    }

    /// Get all channels
    pub fn get_all_channels(&self) -> Vec<(String, String, ChannelEnd)> {
        self.assert_authorized_caller();
        self.channel_module.get_all_channels()
    }

    /// Bind port (reserve a port for this module)
    pub fn bind_port(&mut self, port_id: String) -> bool {
        self.assert_authorized_caller();
        self.channel_module.bind_port(port_id.clone());
        env::log_str(&format!("Port bound: {}", port_id));
        true
    }

    /// Check if port is bound
    pub fn is_port_bound(&self, port_id: String) -> bool {
        self.assert_authorized_caller();
        self.channel_module.is_port_bound(port_id)
    }

    // =============================================================================
    // Validation Functions (called by router for cross-module operations)
    // =============================================================================

    /// Validate packet send (called by router during cross-module operations)
    pub fn validate_send_packet(&self, packet_data: Base64VecU8) -> bool {
        self.assert_authorized_caller();
        
        // Decode packet and validate
        if let Ok(packet_data) = serde_json::from_slice::<PacketData>(&packet_data.0) {
            let packet: Packet = packet_data.into();
            
            // Basic validation
            if packet.source_port.is_empty() || packet.source_channel.is_empty() {
                return false;
            }
            
            // Check if channel exists and is open
            if let Some(channel) = self.channel_module.get_channel(
                packet.source_port.clone(), 
                packet.source_channel.clone()
            ) {
                // Channel should be in OPEN state for packet sending
                matches!(channel.state, crate::modules::ibc::channel::types::State::Open)
            } else {
                false
            }
        } else {
            false
        }
    }

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

    /// Health check for the channel module
    pub fn health_check(&self) -> bool {
        // Check if the channel module is functioning
        true // In a full implementation, would perform actual health checks
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "module_type": "ibc_channel",
            "version": "1.0.0",
            "description": "IBC Channel Module",
            "functions": [
                "channel_open_init",
                "channel_open_try",
                "channel_open_ack", 
                "channel_open_confirm",
                "send_packet",
                "receive_packet",
                "acknowledge_packet",
                "timeout_packet",
                "get_channel",
                "query_packet_commitment",
                "query_packet_acknowledgment",
                "query_packet_receipt",
                "query_next_sequence_recv",
                "query_next_sequence_send",
                "query_next_sequence_ack",
                "get_all_channels",
                "bind_port",
                "is_port_bound",
                "validate_send_packet"
            ]
        })
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
    fn test_channel_contract_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcChannelContract::new(accounts(1), Some(accounts(2)));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, Some(accounts(2)));
    }

    #[test]
    fn test_port_binding() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let mut contract = IbcChannelContract::new(accounts(1), None);
        
        let success = contract.bind_port("transfer".to_string());
        assert!(success);
        
        let is_bound = contract.is_port_bound("transfer".to_string());
        assert!(is_bound);
    }

    #[test]
    fn test_health_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = IbcChannelContract::new(accounts(1), None);
        assert!(contract.health_check());
    }
}