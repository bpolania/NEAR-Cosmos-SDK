use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;

pub type Balance = u128;

pub mod modules;
pub mod types;
pub mod handler;
pub mod crypto;
pub mod contracts;

use modules::bank::BankModule;
use modules::gov::GovernanceModule;
use modules::staking::StakingModule;
use modules::wasm::{WasmModule, WasmMsg, WasmQuery, CodeID, ContractAddress, InstantiateResponse, ExecuteResponse};
use modules::ibc::client::tendermint::{TendermintLightClientModule, Header, Height};
use modules::ibc::connection::{ConnectionModule, ConnectionEnd, Counterparty, Version};
use modules::ibc::connection::types::{MerklePrefix};
use modules::ibc::channel::{ChannelModule, ChannelEnd, Order, Packet, Acknowledgement};
use modules::ibc::channel::types::{PacketCommitment, PacketReceipt};
use modules::ibc::transfer::{TransferModule, FungibleTokenPacketData, DenomTrace};

use handler::{CosmosMessageHandler, HandleResponse, HandleResult, route_cosmos_message, success_result, create_event, validate_cosmos_address, CosmosTransactionHandler, TxProcessingConfig, TxResponse};
use types::cosmos_messages::*;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosContract {
    bank_module: BankModule,
    staking_module: StakingModule,
    governance_module: GovernanceModule,
    wasm_module: WasmModule,
    ibc_client_module: TendermintLightClientModule,
    ibc_connection_module: ConnectionModule,
    ibc_channel_module: ChannelModule,
    ibc_transfer_module: TransferModule,
    tx_config: TxProcessingConfig,
    block_height: u64,
}

#[near_bindgen]
impl CosmosContract {
    #[init]
    pub fn new() -> Self {
        let tx_config = TxProcessingConfig {
            chain_id: "near-cosmos-sdk".to_string(),
            max_gas_per_tx: 10_000_000,
            gas_price: 1,
            verify_signatures: false, // Disabled for development
            check_sequences: false,   // Disabled for development
        };
        
        Self {
            bank_module: BankModule::new(),
            staking_module: StakingModule::new(),
            governance_module: GovernanceModule::new(),
            wasm_module: WasmModule::new(),
            ibc_client_module: TendermintLightClientModule::new(),
            ibc_connection_module: ConnectionModule::new(),
            ibc_channel_module: ChannelModule::new(),
            ibc_transfer_module: TransferModule::new(),
            tx_config,
            block_height: 0,
        }
    }

    // Bank Module Functions
    pub fn transfer(&mut self, receiver: AccountId, amount: Balance) -> String {
        let sender = env::predecessor_account_id();
        self.bank_module.transfer(&sender, &receiver, amount);
        format!("Transferred {} from {} to {}", amount, sender, receiver)
    }

    pub fn mint(&mut self, receiver: AccountId, amount: Balance) -> String {
        self.bank_module.mint(&receiver, amount);
        format!("Minted {} to {}", amount, receiver)
    }

    pub fn get_balance(&self, account: AccountId) -> Balance {
        self.bank_module.get_balance(&account)
    }

    // Staking Module Functions
    pub fn add_validator(&mut self, validator: AccountId) -> String {
        self.staking_module.add_validator(&validator);
        format!("Added validator {}", validator)
    }

    pub fn delegate(&mut self, validator: AccountId, amount: Balance) -> String {
        let delegator = env::predecessor_account_id();
        self.staking_module.delegate(&delegator, &validator, amount, &mut self.bank_module);
        format!("Delegated {} to {} from {}", amount, validator, delegator)
    }

    pub fn undelegate(&mut self, validator: AccountId, amount: Balance) -> String {
        let delegator = env::predecessor_account_id();
        self.staking_module.undelegate(&delegator, &validator, amount, self.block_height);
        format!("Undelegated {} from {} by {}", amount, validator, delegator)
    }

    // Governance Module Functions
    pub fn submit_proposal(&mut self, title: String, description: String, param_key: String, param_value: String) -> u64 {
        let proposer = env::predecessor_account_id();
        let proposal_id = self.governance_module.submit_proposal(&proposer, title, description, param_key, param_value, self.block_height);
        env::log_str(&format!("Submitted proposal {} by {}", proposal_id, proposer));
        proposal_id
    }

    pub fn vote(&mut self, proposal_id: u64, option: u8) -> String {
        let voter = env::predecessor_account_id();
        self.governance_module.vote(&voter, proposal_id, option);
        format!("Voted {} on proposal {} by {}", option, proposal_id, voter)
    }

    pub fn get_parameter(&self, key: String) -> String {
        self.governance_module.get_parameter(&key)
    }

    // Block Processing
    pub fn process_block(&mut self) -> String {
        self.block_height += 1;
        
        // Begin block processing
        self.staking_module.begin_block(self.block_height);
        
        // End block processing
        self.staking_module.end_block(self.block_height, &mut self.bank_module);
        self.governance_module.end_block(self.block_height);
        
        format!("Processed block {}", self.block_height)
    }

    // IBC Client Module Functions
    pub fn ibc_create_client(
        &mut self,
        chain_id: String,
        trust_period: u64,
        unbonding_period: u64,
        max_clock_drift: u64,
        initial_header: Header,
    ) -> String {
        self.ibc_client_module.create_client(chain_id, trust_period, unbonding_period, max_clock_drift, initial_header)
    }

    pub fn ibc_update_client(&mut self, client_id: String, header: Header) -> bool {
        self.ibc_client_module.update_client(client_id, header)
    }

    pub fn ibc_verify_membership(
        &self,
        client_id: String,
        height: u64,
        key: Vec<u8>,
        value: Vec<u8>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_membership(client_id, height, key, value, proof)
    }

    pub fn ibc_verify_non_membership(
        &self,
        client_id: String,
        height: u64,
        key: Vec<u8>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_non_membership(client_id, height, key, proof)
    }

    pub fn ibc_verify_batch_membership(
        &self,
        client_id: String,
        height: u64,
        items: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_batch_membership(client_id, height, items, proof)
    }

    pub fn ibc_verify_mixed_batch_membership(
        &self,
        client_id: String,
        height: u64,
        exist_items: Vec<(Vec<u8>, Vec<u8>)>,
        non_exist_keys: Vec<Vec<u8>>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_mixed_batch_membership(client_id, height, exist_items, non_exist_keys, proof)
    }

    pub fn ibc_verify_compressed_batch_membership(
        &self,
        client_id: String,
        height: u64,
        items: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_compressed_batch_membership(client_id, height, items, proof)
    }

    pub fn ibc_verify_range_membership(
        &self,
        client_id: String,
        height: u64,
        start_key: Vec<u8>,
        end_key: Vec<u8>,
        existence: bool,
        expected_values: Vec<(Vec<u8>, Vec<u8>)>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_range_membership(client_id, height, start_key, end_key, existence, expected_values, proof)
    }

    pub fn ibc_verify_multistore_membership(
        &self,
        client_id: String,
        height: u64,
        store_name: String,
        key: Vec<u8>,
        value: Vec<u8>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_multistore_membership(client_id, height, store_name, key, value, proof)
    }

    pub fn ibc_verify_multistore_batch(
        &self,
        client_id: String,
        height: u64,
        items: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>)>,
    ) -> bool {
        self.ibc_client_module.verify_multistore_batch(client_id, height, items)
    }

    pub fn ibc_get_client_state(&self, client_id: String) -> Option<modules::ibc::client::tendermint::ClientState> {
        self.ibc_client_module.get_client_state(client_id)
    }

    pub fn ibc_get_consensus_state(&self, client_id: String, height: u64) -> Option<modules::ibc::client::tendermint::ConsensusState> {
        self.ibc_client_module.get_consensus_state(client_id, height)
    }

    pub fn ibc_get_latest_height(&self, client_id: String) -> Option<Height> {
        self.ibc_client_module.get_latest_height(client_id)
    }

    pub fn ibc_prune_expired_consensus_state(&mut self, client_id: String, height: u64) -> bool {
        self.ibc_client_module.prune_expired_consensus_state(client_id, height)
    }

    // IBC Connection Module Functions
    pub fn ibc_conn_open_init(
        &mut self,
        client_id: String,
        counterparty_client_id: String,
        counterparty_prefix: Option<Vec<u8>>,
        version: Option<Version>,
        delay_period: u64,
    ) -> String {
        let prefix = counterparty_prefix.unwrap_or_else(|| b"ibc".to_vec());
        let counterparty = Counterparty::new(
            counterparty_client_id,
            None,
            MerklePrefix::new(prefix),
        );
        
        self.ibc_connection_module.conn_open_init(
            client_id,
            counterparty,
            version,
            delay_period,
        )
    }

    #[handle_result]
    pub fn ibc_conn_open_try(
        &mut self,
        previous_connection_id: Option<String>,
        counterparty_client_id: String,
        counterparty_connection_id: String,
        counterparty_prefix: Option<Vec<u8>>,
        delay_period: u64,
        client_id: String,
        client_state_proof: Vec<u8>,
        consensus_state_proof: Vec<u8>,
        connection_proof: Vec<u8>,
        proof_height: u64,
        version: Version,
    ) -> Result<String, String> {
        let prefix = counterparty_prefix.unwrap_or_else(|| b"ibc".to_vec());
        let counterparty = Counterparty::new(
            counterparty_client_id,
            Some(counterparty_connection_id),
            MerklePrefix::new(prefix),
        );

        self.ibc_connection_module.conn_open_try(
            previous_connection_id,
            counterparty,
            delay_period,
            client_id,
            client_state_proof,
            consensus_state_proof,
            connection_proof,
            proof_height,
            version,
        )
    }

    #[handle_result]
    pub fn ibc_conn_open_ack(
        &mut self,
        connection_id: String,
        counterparty_connection_id: String,
        version: Version,
        client_state_proof: Vec<u8>,
        connection_proof: Vec<u8>,
        consensus_state_proof: Vec<u8>,
        proof_height: u64,
    ) -> Result<(), String> {
        self.ibc_connection_module.conn_open_ack(
            connection_id,
            counterparty_connection_id,
            version,
            client_state_proof,
            connection_proof,
            consensus_state_proof,
            proof_height,
        )
    }

    #[handle_result]
    pub fn ibc_conn_open_confirm(
        &mut self,
        connection_id: String,
        connection_proof: Vec<u8>,
        proof_height: u64,
    ) -> Result<(), String> {
        self.ibc_connection_module.conn_open_confirm(
            connection_id,
            connection_proof,
            proof_height,
        )
    }

    pub fn ibc_get_connection(&self, connection_id: String) -> Option<ConnectionEnd> {
        self.ibc_connection_module.get_connection(connection_id)
    }

    pub fn ibc_get_connection_ids(&self) -> Vec<String> {
        self.ibc_connection_module.get_connection_ids()
    }

    pub fn ibc_is_connection_open(&self, connection_id: String) -> bool {
        self.ibc_connection_module.is_connection_open(&connection_id)
    }

    // IBC Channel Module Functions
    pub fn ibc_chan_open_init(
        &mut self,
        port_id: String,
        order: u8, // 0 = Unordered, 1 = Ordered
        connection_hops: Vec<String>,
        counterparty_port_id: String,
        version: String,
    ) -> String {
        let channel_order = if order == 1 { Order::Ordered } else { Order::Unordered };
        
        self.ibc_channel_module.chan_open_init(
            port_id,
            channel_order,
            connection_hops,
            counterparty_port_id,
            version,
        )
    }

    #[handle_result]
    pub fn ibc_chan_open_try(
        &mut self,
        port_id: String,
        previous_channel_id: Option<String>,
        order: u8, // 0 = Unordered, 1 = Ordered
        connection_hops: Vec<String>,
        counterparty_port_id: String,
        counterparty_channel_id: String,
        version: String,
        counterparty_version: String,
        channel_proof: Vec<u8>,
        proof_height: u64,
    ) -> Result<String, String> {
        let channel_order = if order == 1 { Order::Ordered } else { Order::Unordered };
        
        self.ibc_channel_module.chan_open_try(
            port_id,
            previous_channel_id,
            channel_order,
            connection_hops,
            counterparty_port_id,
            counterparty_channel_id,
            version,
            counterparty_version,
            channel_proof,
            proof_height,
        )
    }

    #[handle_result]
    pub fn ibc_chan_open_ack(
        &mut self,
        port_id: String,
        channel_id: String,
        counterparty_channel_id: String,
        counterparty_version: String,
        channel_proof: Vec<u8>,
        proof_height: u64,
    ) -> Result<(), String> {
        self.ibc_channel_module.chan_open_ack(
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
            channel_proof,
            proof_height,
        )
    }

    #[handle_result]
    pub fn ibc_chan_open_confirm(
        &mut self,
        port_id: String,
        channel_id: String,
        channel_proof: Vec<u8>,
        proof_height: u64,
    ) -> Result<(), String> {
        self.ibc_channel_module.chan_open_confirm(
            port_id,
            channel_id,
            channel_proof,
            proof_height,
        )
    }

    #[handle_result]
    pub fn ibc_send_packet(
        &mut self,
        source_port: String,
        source_channel: String,
        timeout_height_revision: u64,
        timeout_height_value: u64,
        timeout_timestamp: u64,
        data: Vec<u8>,
    ) -> Result<u64, String> {
        let timeout_height = modules::ibc::channel::Height::new(timeout_height_revision, timeout_height_value);
        
        self.ibc_channel_module.send_packet(
            source_port,
            source_channel,
            timeout_height,
            timeout_timestamp,
            data,
        )
    }

    #[handle_result]
    pub fn ibc_recv_packet(
        &mut self,
        sequence: u64,
        source_port: String,
        source_channel: String,
        destination_port: String,
        destination_channel: String,
        data: Vec<u8>,
        timeout_height_revision: u64,
        timeout_height_value: u64,
        timeout_timestamp: u64,
        packet_proof: Vec<u8>,
        proof_height: u64,
    ) -> Result<(), String> {
        let timeout_height = modules::ibc::channel::Height::new(timeout_height_revision, timeout_height_value);
        
        let packet = Packet::new(
            sequence,
            source_port,
            source_channel,
            destination_port,
            destination_channel,
            data,
            timeout_height,
            timeout_timestamp,
        );

        self.ibc_channel_module.recv_packet(packet, packet_proof, proof_height)
    }

    #[handle_result]
    pub fn ibc_acknowledge_packet(
        &mut self,
        sequence: u64,
        source_port: String,
        source_channel: String,
        destination_port: String,
        destination_channel: String,
        data: Vec<u8>,
        timeout_height_revision: u64,
        timeout_height_value: u64,
        timeout_timestamp: u64,
        acknowledgement_data: Vec<u8>,
        ack_proof: Vec<u8>,
        proof_height: u64,
    ) -> Result<(), String> {
        let timeout_height = modules::ibc::channel::Height::new(timeout_height_revision, timeout_height_value);
        
        let packet = Packet::new(
            sequence,
            source_port,
            source_channel,
            destination_port,
            destination_channel,
            data,
            timeout_height,
            timeout_timestamp,
        );

        let acknowledgement = Acknowledgement::new(acknowledgement_data);

        self.ibc_channel_module.acknowledge_packet(packet, acknowledgement, ack_proof, proof_height)
    }

    pub fn ibc_get_channel(&self, port_id: String, channel_id: String) -> Option<ChannelEnd> {
        self.ibc_channel_module.get_channel(port_id, channel_id)
    }

    pub fn ibc_is_channel_open(&self, port_id: String, channel_id: String) -> bool {
        self.ibc_channel_module.is_channel_open(&port_id, &channel_id)
    }

    pub fn ibc_get_next_sequence_send(&self, port_id: String, channel_id: String) -> u64 {
        self.ibc_channel_module.get_next_sequence_send(&port_id, &channel_id)
    }

    pub fn ibc_get_next_sequence_recv(&self, port_id: String, channel_id: String) -> u64 {
        self.ibc_channel_module.get_next_sequence_recv(&port_id, &channel_id)
    }

    pub fn ibc_get_packet_commitment(&self, port_id: String, channel_id: String, sequence: u64) -> Option<PacketCommitment> {
        self.ibc_channel_module.get_packet_commitment(&port_id, &channel_id, sequence)
    }

    pub fn ibc_get_packet_receipt(&self, port_id: String, channel_id: String, sequence: u64) -> Option<PacketReceipt> {
        self.ibc_channel_module.get_packet_receipt(&port_id, &channel_id, sequence)
    }

    pub fn ibc_get_packet_acknowledgement(&self, port_id: String, channel_id: String, sequence: u64) -> Option<Acknowledgement> {
        self.ibc_channel_module.get_packet_acknowledgement(&port_id, &channel_id, sequence)
    }

    pub fn ibc_create_success_acknowledgement(&self, result: Vec<u8>) -> Acknowledgement {
        self.ibc_channel_module.create_success_acknowledgement(result)
    }

    pub fn ibc_create_error_acknowledgement(&self, error: String) -> Acknowledgement {
        self.ibc_channel_module.create_error_acknowledgement(error)
    }

    pub fn ibc_is_acknowledgement_success(&self, ack: Acknowledgement) -> bool {
        self.ibc_channel_module.is_acknowledgement_success(&ack)
    }

    pub fn ibc_create_packet_commitment(&self, data: Vec<u8>) -> PacketCommitment {
        self.ibc_channel_module.create_packet_commitment(data)
    }

    pub fn ibc_is_timeout_height_zero(&self, height_revision: u64, height_value: u64) -> bool {
        let height = modules::ibc::channel::types::Height::new(height_revision, height_value);
        self.ibc_channel_module.is_timeout_height_zero(&height)
    }

    // ICS-20 Fungible Token Transfer Functions
    
    /// Send a cross-chain token transfer via IBC
    /// 
    /// # Arguments
    /// * `source_channel` - Channel to send the transfer through
    /// * `token_denom` - Token denomination to transfer
    /// * `amount` - Amount to transfer
    /// * `receiver` - Destination address on receiving chain
    /// * `timeout_height_revision` - Timeout height revision number
    /// * `timeout_height_value` - Timeout height value
    /// * `timeout_timestamp` - Timeout timestamp (nanoseconds)
    /// * `memo` - Optional memo string
    /// 
    /// # Returns
    /// * Packet sequence number on success
    #[handle_result]
    pub fn ibc_transfer(
        &mut self,
        source_channel: String,
        token_denom: String,
        amount: Balance,
        receiver: String,
        timeout_height_revision: u64,
        timeout_height_value: u64,
        timeout_timestamp: u64,
        memo: Option<String>,
    ) -> Result<u64, String> {
        let sender = env::predecessor_account_id().to_string();
        let timeout_height = modules::ibc::channel::Height::new(timeout_height_revision, timeout_height_value);
        
        self.ibc_transfer_module.send_transfer(
            &mut self.ibc_channel_module,
            &mut self.bank_module,
            "transfer".to_string(),
            source_channel,
            token_denom,
            amount,
            sender,
            receiver,
            timeout_height,
            timeout_timestamp,
            memo,
        ).map_err(|e| format!("Transfer failed: {:?}", e))
    }

    /// Get denomination trace information
    /// 
    /// # Arguments
    /// * `trace_hash` - Hash of the denomination trace
    /// 
    /// # Returns
    /// * DenomTrace if found
    pub fn ibc_get_denom_trace(&self, trace_hash: String) -> Option<DenomTrace> {
        self.ibc_transfer_module.get_denom_trace(&trace_hash)
    }

    /// Get trace path by IBC denomination
    /// 
    /// # Arguments
    /// * `ibc_denom` - IBC denomination (format: "ibc/{hash}")
    /// 
    /// # Returns
    /// * Trace path if found
    pub fn ibc_get_trace_path(&self, ibc_denom: String) -> Option<String> {
        self.ibc_transfer_module.get_trace_path(&ibc_denom)
    }

    /// Get escrowed token amount for a channel
    /// 
    /// # Arguments
    /// * `port_id` - Port identifier
    /// * `channel_id` - Channel identifier
    /// * `denom` - Token denomination
    /// 
    /// # Returns
    /// * Escrowed amount
    pub fn ibc_get_escrowed_amount(&self, port_id: String, channel_id: String, denom: String) -> Balance {
        self.ibc_transfer_module.get_escrowed_amount(&port_id, &channel_id, &denom)
    }

    /// Get voucher token supply for a denomination
    /// 
    /// # Arguments
    /// * `denom` - Token denomination
    /// 
    /// # Returns
    /// * Total voucher supply
    pub fn ibc_get_voucher_supply(&self, denom: String) -> Balance {
        self.ibc_transfer_module.get_voucher_supply(&denom)
    }

    /// Check if a denomination is from source zone (native to this chain)
    /// 
    /// # Arguments
    /// * `port_id` - Port identifier
    /// * `channel_id` - Channel identifier
    /// * `denom` - Token denomination
    /// 
    /// # Returns
    /// * True if token is from source zone
    pub fn ibc_is_source_zone(&self, port_id: String, channel_id: String, denom: String) -> bool {
        self.ibc_transfer_module.is_source_zone(&port_id, &channel_id, &denom)
    }

    /// Create IBC denomination for outgoing transfers
    /// 
    /// # Arguments
    /// * `port_id` - Port identifier
    /// * `channel_id` - Channel identifier
    /// * `denom` - Original denomination
    /// 
    /// # Returns
    /// * IBC denomination string
    pub fn ibc_create_ibc_denom(&self, port_id: String, channel_id: String, denom: String) -> String {
        self.ibc_transfer_module.create_ibc_denom(&port_id, &channel_id, &denom)
    }

    /// Validate transfer parameters before execution
    /// 
    /// # Arguments
    /// * `source_port` - Source port identifier
    /// * `source_channel` - Source channel identifier  
    /// * `denom` - Token denomination
    /// * `amount` - Transfer amount
    /// * `sender` - Sender address
    /// 
    /// # Returns
    /// * Success or error message
    #[handle_result]
    pub fn ibc_validate_transfer(
        &self,
        source_port: String,
        source_channel: String,
        denom: String,
        amount: Balance,
        sender: String,
    ) -> Result<(), String> {
        self.ibc_transfer_module.validate_transfer(
            &self.ibc_channel_module,
            &self.bank_module,
            &source_port,
            &source_channel,
            &denom,
            amount,
            &sender,
        ).map_err(|e| format!("Validation failed: {:?}", e))
    }

    /// Process received IBC transfer packet
    /// 
    /// This function is called internally when receiving ICS-20 packets.
    /// It's exposed for testing and debugging purposes.
    /// 
    /// # Arguments
    /// * `packet_data` - Raw packet data bytes
    /// 
    /// # Returns
    /// * Success or acknowledgement data
    #[handle_result]
    pub fn ibc_process_transfer_packet(&mut self, packet_data: Vec<u8>) -> Result<Vec<u8>, String> {
        // Parse packet data
        let _transfer_data = FungibleTokenPacketData::from_bytes(&packet_data)
            .map_err(|e| format!("Invalid packet data: {:?}", e))?;

        // Create a mock packet for processing (in real implementation this would come from IBC channel)
        let packet = modules::ibc::channel::Packet::new(
            1, // sequence
            "transfer".to_string(),
            "channel-0".to_string(),
            "transfer".to_string(),
            "channel-1".to_string(),
            packet_data,
            modules::ibc::channel::Height::new(1, 1000),
            0,
        );

        let ack = self.ibc_transfer_module.receive_transfer(
            &self.ibc_channel_module,
            &mut self.bank_module,
            &packet,
        ).map_err(|e| format!("Transfer processing failed: {:?}", e))?;

        Ok(ack.data)
    }

    /// Register a denomination trace
    /// 
    /// # Arguments
    /// * `path` - Full trace path (e.g., "transfer/channel-0/uatom")
    /// 
    /// # Returns
    /// * IBC denomination (ibc/{hash})
    #[handle_result]
    pub fn ibc_register_denom_trace(&mut self, path: String) -> Result<String, String> {
        let denom_trace = DenomTrace::from_path(&path)
            .map_err(|e| format!("Invalid trace path: {:?}", e))?;
        
        Ok(self.ibc_transfer_module.register_denom_trace(denom_trace))
    }

    /// Handle a Cosmos SDK message using the message router
    /// 
    /// # Arguments
    /// * `msg_type` - Cosmos SDK message type URL (e.g., "/cosmos.bank.v1beta1.MsgSend")
    /// * `msg_data` - Base64-encoded message data
    /// 
    /// # Returns
    /// * HandleResponse with result code, data, log, and events
    pub fn handle_cosmos_msg(&mut self, msg_type: String, msg_data: Base64VecU8) -> HandleResponse {
        // Use the message router to handle the message
        route_cosmos_message(self, msg_type, msg_data)
    }

    // ========================================================================
    // COSMOS SDK PUBLIC API METHODS (Phase 2 Week 4.1)
    // ========================================================================

    /// Create a transaction handler on demand
    fn create_transaction_handler(&self) -> CosmosTransactionHandler {
        CosmosTransactionHandler::new(self.tx_config.clone())
    }

    /// Broadcast a transaction synchronously
    /// 
    /// This is the primary method for submitting Cosmos SDK transactions to the NEAR contract.
    /// It processes the transaction immediately and returns the result.
    /// 
    /// # Arguments
    /// * `tx_bytes` - Base64 encoded serialized Cosmos transaction
    /// 
    /// # Returns
    /// * `TxResponse` - Complete ABCI-compatible transaction response
    pub fn broadcast_tx_sync(&mut self, tx_bytes: Base64VecU8) -> TxResponse {
        let mut handler = self.create_transaction_handler();
        match handler.process_transaction(tx_bytes.0, self) {
            Ok(response) => response,
            Err(error) => TxResponse::error(error, None),
        }
    }

    /// Simulate a transaction without executing it
    /// 
    /// Performs all validation and gas estimation without modifying state.
    /// Useful for gas estimation, transaction validation, and dApp UX.
    /// 
    /// # Arguments
    /// * `tx_bytes` - Base64 encoded serialized Cosmos transaction
    /// 
    /// # Returns
    /// * `TxResponse` - Simulation response with gas usage and validation results
    pub fn simulate_tx(&mut self, tx_bytes: Base64VecU8) -> TxResponse {
        let mut handler = self.create_transaction_handler();
        match handler.simulate_transaction(tx_bytes.0) {
            Ok(response) => response,
            Err(error) => TxResponse::error(error, None),
        }
    }

    /// Broadcast transaction asynchronously (same as sync for NEAR)
    /// 
    /// On NEAR, all transactions are processed synchronously, so this method
    /// behaves identically to broadcast_tx_sync for compatibility.
    /// 
    /// # Arguments
    /// * `tx_bytes` - Base64 encoded serialized Cosmos transaction
    /// 
    /// # Returns
    /// * `TxResponse` - Complete ABCI-compatible transaction response
    pub fn broadcast_tx_async(&mut self, tx_bytes: Base64VecU8) -> TxResponse {
        self.broadcast_tx_sync(tx_bytes)
    }

    /// Broadcast transaction and wait for commit (same as sync for NEAR)
    /// 
    /// On NEAR, transactions are immediately included in blocks, so this method
    /// behaves identically to broadcast_tx_sync for compatibility.
    /// 
    /// # Arguments
    /// * `tx_bytes` - Base64 encoded serialized Cosmos transaction
    /// 
    /// # Returns
    /// * `TxResponse` - Complete ABCI-compatible transaction response with block inclusion
    pub fn broadcast_tx_commit(&mut self, tx_bytes: Base64VecU8) -> TxResponse {
        let mut response = self.broadcast_tx_sync(tx_bytes);
        // On NEAR, we can set the height to current block since it's immediately included
        response.height = self.block_height.to_string();
        response
    }

    /// Get transaction by hash (placeholder implementation)
    /// 
    /// In a full implementation, this would query transaction history.
    /// Currently returns error response as transaction storage is not implemented.
    /// 
    /// # Arguments
    /// * `hash` - Transaction hash to lookup
    /// 
    /// # Returns
    /// * `TxResponse` - Transaction response if found, error response otherwise
    pub fn get_tx(&self, _hash: String) -> TxResponse {
        // TODO: Implement transaction storage and retrieval
        // This would require storing transactions in contract state
        use crate::handler::TxProcessingError;
        TxResponse::error(TxProcessingError::TransactionNotFound, None)
    }

    /// Update transaction processing configuration
    /// 
    /// Allows updating chain ID, gas limits, and other processing parameters.
    /// 
    /// # Arguments
    /// * `config` - New transaction processing configuration
    pub fn update_tx_config(&mut self, config: TxProcessingConfig) {
        self.tx_config = config;
    }

    /// Get current transaction processing configuration
    /// 
    /// # Returns
    /// * `TxProcessingConfig` - Current configuration
    pub fn get_tx_config(&self) -> TxProcessingConfig {
        self.tx_config.clone()
    }

    // CosmWasm Module Functions
    /// Store WASM code and return CodeID
    #[handle_result]
    pub fn wasm_store_code(
        &mut self,
        wasm_byte_code: Vec<u8>,
        source: Option<String>,
        builder: Option<String>,
        instantiate_permission: Option<modules::wasm::AccessConfig>,
    ) -> Result<CodeID, String> {
        let sender = env::predecessor_account_id();
        self.wasm_module.store_code(&sender, wasm_byte_code, source, builder, instantiate_permission)
    }

    /// Instantiate a contract from stored code
    #[handle_result]
    pub fn wasm_instantiate(
        &mut self,
        code_id: CodeID,
        msg: Vec<u8>,
        funds: Vec<modules::wasm::Coin>,
        label: String,
        admin: Option<AccountId>,
    ) -> Result<InstantiateResponse, String> {
        let sender = env::predecessor_account_id();
        self.wasm_module.instantiate_contract(&sender, code_id, msg, funds, label, admin)
    }

    /// Execute a message on a contract
    #[handle_result]
    pub fn wasm_execute(
        &mut self,
        contract_addr: ContractAddress,
        msg: Vec<u8>,
        funds: Vec<modules::wasm::Coin>,
    ) -> Result<ExecuteResponse, String> {
        let sender = env::predecessor_account_id();
        self.wasm_module.execute_contract(&sender, &contract_addr, msg, funds)
    }

    /// Query a contract
    #[handle_result]
    pub fn wasm_smart_query(&self, contract_addr: ContractAddress, msg: Vec<u8>) -> Result<Vec<u8>, String> {
        self.wasm_module.query_contract(&contract_addr, msg)
    }

    /// Get contract info
    pub fn wasm_contract_info(&self, address: ContractAddress) -> Option<modules::wasm::ContractInfo> {
        self.wasm_module.get_contract_info(&address)
    }

    /// Get code info
    pub fn wasm_code_info(&self, code_id: CodeID) -> Option<modules::wasm::CodeInfo> {
        self.wasm_module.get_code_info(code_id)
    }

    /// List stored codes
    pub fn wasm_list_codes(&self, start_after: Option<CodeID>, limit: Option<u32>) -> Vec<modules::wasm::CodeInfo> {
        self.wasm_module.list_codes(start_after, limit)
    }

    /// List contracts by code ID
    pub fn wasm_list_contracts_by_code(
        &self,
        code_id: CodeID,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<modules::wasm::ContractInfo> {
        self.wasm_module.list_contracts_by_code(code_id, start_after, limit)
    }

    // View functions
    pub fn get_block_height(&self) -> u64 {
        self.block_height
    }
}

// Implementation of CosmosMessageHandler trait for the main contract
impl CosmosMessageHandler for CosmosContract {
    // Bank module handlers
    fn handle_msg_send(&mut self, msg: MsgSend) -> handler::MessageResult<HandleResult> {
        // Validate addresses
        validate_cosmos_address(&msg.from_address)?;
        validate_cosmos_address(&msg.to_address)?;
        
        if msg.amount.is_empty() {
            return Err(handler::ContractError::Custom("Empty amount".to_string()));
        }

        // For now, handle only the first coin and convert to NEAR balance
        let coin = &msg.amount[0];
        let amount: Balance = coin.amount.parse()
            .map_err(|_| handler::ContractError::Custom("Invalid amount format".to_string()))?;

        // Convert addresses to NEAR AccountId format (simplified for now)
        let from_account = msg.from_address.parse::<AccountId>()
            .unwrap_or_else(|_| env::predecessor_account_id());
        let to_account = msg.to_address.parse::<AccountId>()
            .unwrap_or_else(|_| "default.near".parse().unwrap());

        // Execute the transfer using the bank module
        self.bank_module.transfer(&from_account, &to_account, amount);

        let log_msg = format!("Transferred {} from {} to {}", 
            format_coins(&msg.amount), msg.from_address, msg.to_address);

        let events = vec![create_event("transfer", vec![
            ("sender", &msg.from_address),
            ("recipient", &msg.to_address),
            ("amount", &format_coins(&msg.amount)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_multi_send(&mut self, msg: MsgMultiSend) -> handler::MessageResult<HandleResult> {
        if msg.inputs.is_empty() || msg.outputs.is_empty() {
            return Err(handler::ContractError::Custom("Empty inputs or outputs".to_string()));
        }

        // Validate all addresses
        for input in &msg.inputs {
            validate_cosmos_address(&input.address)?;
        }
        for output in &msg.outputs {
            validate_cosmos_address(&output.address)?;
        }

        // For simplicity, we'll log the operation but not implement full multi-send logic
        let log_msg = format!("Multi-send executed: {} inputs, {} outputs", 
            msg.inputs.len(), msg.outputs.len());

        let events = vec![create_event("multi_send", vec![
            ("input_count", &msg.inputs.len().to_string()),
            ("output_count", &msg.outputs.len().to_string()),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_burn(&mut self, msg: MsgBurn) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.from_address)?;
        
        if msg.amount.is_empty() {
            return Err(handler::ContractError::Custom("Empty amount".to_string()));
        }

        let log_msg = format!("Burned {} from {}", 
            format_coins(&msg.amount), msg.from_address);

        let events = vec![create_event("burn", vec![
            ("burner", &msg.from_address),
            ("amount", &format_coins(&msg.amount)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    // Staking module handlers
    fn handle_msg_delegate(&mut self, msg: MsgDelegate) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.delegator_address)?;
        validate_cosmos_address(&msg.validator_address)?;

        let amount: Balance = msg.amount.amount.parse()
            .map_err(|_| handler::ContractError::Custom("Invalid amount format".to_string()))?;

        // Convert addresses
        let delegator = msg.delegator_address.parse::<AccountId>()
            .unwrap_or_else(|_| env::predecessor_account_id());
        let validator = msg.validator_address.parse::<AccountId>()
            .unwrap_or_else(|_| "validator.near".parse().unwrap());

        // Execute delegation using staking module
        self.staking_module.delegate(&delegator, &validator, amount, &mut self.bank_module);

        let log_msg = format!("Delegated {} from {} to {}", 
            format!("{}{}", msg.amount.amount, msg.amount.denom),
            msg.delegator_address, 
            msg.validator_address);

        let events = vec![create_event("delegate", vec![
            ("delegator", &msg.delegator_address),
            ("validator", &msg.validator_address),
            ("amount", &format!("{}{}", msg.amount.amount, msg.amount.denom)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_undelegate(&mut self, msg: MsgUndelegate) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.delegator_address)?;
        validate_cosmos_address(&msg.validator_address)?;

        let amount: Balance = msg.amount.amount.parse()
            .map_err(|_| handler::ContractError::Custom("Invalid amount format".to_string()))?;

        let delegator = msg.delegator_address.parse::<AccountId>()
            .unwrap_or_else(|_| env::predecessor_account_id());
        let validator = msg.validator_address.parse::<AccountId>()
            .unwrap_or_else(|_| "validator.near".parse().unwrap());

        // Execute undelegation
        self.staking_module.undelegate(&delegator, &validator, amount, self.block_height);

        let log_msg = format!("Undelegated {} from {} by {}", 
            format!("{}{}", msg.amount.amount, msg.amount.denom),
            msg.validator_address, 
            msg.delegator_address);

        let events = vec![create_event("undelegate", vec![
            ("delegator", &msg.delegator_address),
            ("validator", &msg.validator_address),
            ("amount", &format!("{}{}", msg.amount.amount, msg.amount.denom)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_begin_redelegate(&mut self, msg: MsgBeginRedelegate) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.delegator_address)?;
        validate_cosmos_address(&msg.validator_src_address)?;
        validate_cosmos_address(&msg.validator_dst_address)?;

        let log_msg = format!("Redelegated {} from {} to {} by {}", 
            format!("{}{}", msg.amount.amount, msg.amount.denom),
            msg.validator_src_address,
            msg.validator_dst_address,
            msg.delegator_address);

        let events = vec![create_event("redelegate", vec![
            ("delegator", &msg.delegator_address),
            ("validator_src", &msg.validator_src_address),
            ("validator_dst", &msg.validator_dst_address),
            ("amount", &format!("{}{}", msg.amount.amount, msg.amount.denom)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_create_validator(&mut self, msg: MsgCreateValidator) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.delegator_address)?;
        validate_cosmos_address(&msg.validator_address)?;

        let validator = msg.validator_address.parse::<AccountId>()
            .unwrap_or_else(|_| "new-validator.near".parse().unwrap());

        // Add validator using staking module
        self.staking_module.add_validator(&validator);

        let log_msg = format!("Created validator {} with self-delegation {}{}", 
            msg.validator_address,
            msg.value.amount,
            msg.value.denom);

        let events = vec![create_event("create_validator", vec![
            ("validator", &msg.validator_address),
            ("moniker", &msg.description.moniker),
            ("commission_rate", &msg.commission.rate),
            ("self_delegation", &format!("{}{}", msg.value.amount, msg.value.denom)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_edit_validator(&mut self, msg: MsgEditValidator) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.validator_address)?;

        let log_msg = format!("Edited validator {}", msg.validator_address);

        let mut attributes = vec![("validator", msg.validator_address.as_str())];
        if let Some(ref commission_rate) = msg.commission_rate {
            attributes.push(("commission_rate", commission_rate));
        }

        let events = vec![create_event("edit_validator", attributes)];

        Ok(success_result(&log_msg, events))
    }

    // Governance module handlers
    fn handle_msg_submit_proposal(&mut self, msg: MsgSubmitProposal) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.proposer)?;

        let proposer = msg.proposer.parse::<AccountId>()
            .unwrap_or_else(|_| env::predecessor_account_id());

        // For now, submit a simple text proposal
        let proposal_id = self.governance_module.submit_proposal(
            &proposer,
            "Cosmos SDK Proposal".to_string(),
            "Proposal submitted via Cosmos SDK interface".to_string(),
            "param_key".to_string(),
            "param_value".to_string(),
            self.block_height,
        );

        let log_msg = format!("Submitted proposal {} by {}", proposal_id, msg.proposer);

        let events = vec![create_event("submit_proposal", vec![
            ("proposal_id", &proposal_id.to_string()),
            ("proposer", &msg.proposer),
            ("initial_deposit", &format_coins(&msg.initial_deposit)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_vote(&mut self, msg: MsgVote) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.voter)?;

        let voter = msg.voter.parse::<AccountId>()
            .unwrap_or_else(|_| env::predecessor_account_id());

        // Convert VoteOption to u8 for the governance module
        let option = match msg.option {
            VoteOption::Yes => 1u8,
            VoteOption::No => 2u8,
            VoteOption::Abstain => 3u8,
            VoteOption::NoWithVeto => 4u8,
            VoteOption::Unspecified => 0u8,
        };

        self.governance_module.vote(&voter, msg.proposal_id, option);

        let log_msg = format!("Vote cast by {} on proposal {} with option {:?}", 
            msg.voter, msg.proposal_id, msg.option);

        let events = vec![create_event("proposal_vote", vec![
            ("proposal_id", &msg.proposal_id.to_string()),
            ("voter", &msg.voter),
            ("option", &format!("{:?}", msg.option)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_vote_weighted(&mut self, msg: MsgVoteWeighted) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.voter)?;

        let log_msg = format!("Weighted vote cast by {} on proposal {} with {} options", 
            msg.voter, msg.proposal_id, msg.options.len());

        let events = vec![create_event("proposal_vote_weighted", vec![
            ("proposal_id", &msg.proposal_id.to_string()),
            ("voter", &msg.voter),
            ("option_count", &msg.options.len().to_string()),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_deposit(&mut self, msg: MsgDeposit) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.depositor)?;

        let log_msg = format!("Deposit made by {} on proposal {} with amount {}", 
            msg.depositor, msg.proposal_id, format_coins(&msg.amount));

        let events = vec![create_event("proposal_deposit", vec![
            ("proposal_id", &msg.proposal_id.to_string()),
            ("depositor", &msg.depositor),
            ("amount", &format_coins(&msg.amount)),
        ])];

        Ok(success_result(&log_msg, events))
    }

    // IBC module handlers
    fn handle_msg_transfer(&mut self, msg: MsgTransfer) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.sender)?;
        validate_cosmos_address(&msg.receiver)?;

        let amount: Balance = msg.token.amount.parse()
            .map_err(|_| handler::ContractError::Custom("Invalid amount format".to_string()))?;

        // Use the IBC transfer module
        let timeout_height = modules::ibc::channel::Height::new(
            msg.timeout_height.revision_number, 
            msg.timeout_height.revision_height
        );

        let _sequence = self.ibc_transfer_module.send_transfer(
            &mut self.ibc_channel_module,
            &mut self.bank_module,
            msg.source_port.clone(),
            msg.source_channel.clone(),
            msg.token.denom.clone(),
            amount,
            msg.sender.clone(),
            msg.receiver.clone(),
            timeout_height,
            msg.timeout_timestamp,
            None, // memo
        ).map_err(|e| handler::ContractError::Custom(format!("IBC transfer failed: {:?}", e)))?;

        let log_msg = format!("IBC transfer {} from {} to {} via {}", 
            format!("{}{}", msg.token.amount, msg.token.denom),
            msg.sender,
            msg.receiver,
            msg.source_channel);

        let events = vec![create_event("ibc_transfer", vec![
            ("sender", &msg.sender),
            ("receiver", &msg.receiver),
            ("amount", &format!("{}{}", msg.token.amount, msg.token.denom)),
            ("source_port", &msg.source_port),
            ("source_channel", &msg.source_channel),
        ])];

        Ok(success_result(&log_msg, events))
    }

    // IBC channel handlers (simplified implementations)
    fn handle_msg_channel_open_init(&mut self, msg: MsgChannelOpenInit) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.signer)?;

        let log_msg = format!("Channel open init on port {}", msg.port_id);

        let events = vec![create_event("channel_open_init", vec![
            ("port_id", &msg.port_id),
            ("signer", &msg.signer),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_channel_open_try(&mut self, msg: MsgChannelOpenTry) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.signer)?;

        let log_msg = format!("Channel open try on port {} with desired channel {}", 
            msg.port_id, msg.desired_channel_id);

        let events = vec![create_event("channel_open_try", vec![
            ("port_id", &msg.port_id),
            ("desired_channel_id", &msg.desired_channel_id),
            ("signer", &msg.signer),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_recv_packet(&mut self, msg: MsgRecvPacket) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.signer)?;

        let log_msg = format!("Packet received by {}", msg.signer);

        let events = vec![create_event("recv_packet", vec![
            ("signer", &msg.signer),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_acknowledgement(&mut self, msg: MsgAcknowledgement) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.signer)?;

        let log_msg = format!("Packet acknowledged by {}", msg.signer);

        let events = vec![create_event("acknowledge_packet", vec![
            ("signer", &msg.signer),
        ])];

        Ok(success_result(&log_msg, events))
    }

    fn handle_msg_timeout(&mut self, msg: MsgTimeout) -> handler::MessageResult<HandleResult> {
        validate_cosmos_address(&msg.signer)?;

        let log_msg = format!("Packet timeout handled by {}", msg.signer);

        let events = vec![create_event("timeout_packet", vec![
            ("signer", &msg.signer),
            ("next_sequence_recv", &msg.next_sequence_recv.to_string()),
        ])];

        Ok(success_result(&log_msg, events))
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use serde_json;

    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(predecessor);
        builder
    }

    #[test]
    fn test_handle_cosmos_msg_send() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();
        
        // First, mint some tokens to alice.near so she has balance for the transfer
        contract.mint("alice.near".parse().unwrap(), 2000000);

        // Create a MsgSend
        let msg = MsgSend {
            from_address: "alice.near".to_string(),
            to_address: "bob.near".to_string(),
            amount: vec![Coin::new("near", "1000000")],
        };

        // Serialize to JSON bytes
        let msg_data = serde_json::to_vec(&msg).unwrap();
        let msg_data_b64 = Base64VecU8(msg_data);

        // Handle the message
        let response = contract.handle_cosmos_msg(
            type_urls::MSG_SEND.to_string(),
            msg_data_b64,
        );

        // Verify response
        assert_eq!(response.code, 0);
        assert!(response.log.contains("Transferred"));
        assert!(response.log.contains("alice.near"));
        assert!(response.log.contains("bob.near"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "transfer");
    }

    #[test]
    fn test_handle_cosmos_msg_delegate() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();
        
        // First add a validator and mint tokens to alice
        contract.add_validator("validator.near".parse().unwrap());
        contract.mint("alice.near".parse().unwrap(), 10000000);

        // Create a MsgDelegate
        let msg = MsgDelegate {
            delegator_address: "alice.near".to_string(),
            validator_address: "validator.near".to_string(),
            amount: Coin::new("near", "5000000"),
        };

        let msg_data = serde_json::to_vec(&msg).unwrap();
        let msg_data_b64 = Base64VecU8(msg_data);

        let response = contract.handle_cosmos_msg(
            type_urls::MSG_DELEGATE.to_string(),
            msg_data_b64,
        );

        assert_eq!(response.code, 0);
        assert!(response.log.contains("Delegated"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "delegate");
    }

    #[test]
    fn test_handle_cosmos_msg_vote() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();
        
        // First create a proposal so we can vote on it
        let proposal_id = contract.submit_proposal(
            "Test Proposal".to_string(),
            "A test proposal".to_string(),
            "test_param".to_string(),
            "test_value".to_string(),
        );

        let msg = MsgVote {
            proposal_id,
            voter: "alice.near".to_string(),
            option: VoteOption::Yes,
        };

        let msg_data = serde_json::to_vec(&msg).unwrap();
        let msg_data_b64 = Base64VecU8(msg_data);

        let response = contract.handle_cosmos_msg(
            type_urls::MSG_VOTE.to_string(),
            msg_data_b64,
        );

        assert_eq!(response.code, 0);
        assert!(response.log.contains("Vote cast"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "proposal_vote");
    }

    #[test]
    fn test_handle_cosmos_msg_invalid_type() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();

        let response = contract.handle_cosmos_msg(
            "/invalid.message.Type".to_string(),
            Base64VecU8(vec![1, 2, 3]),
        );

        assert_eq!(response.code, 1);
        assert!(response.log.contains("Invalid message type"));
    }

    #[test]
    fn test_handle_cosmos_msg_invalid_data() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();

        // Send invalid JSON data
        let response = contract.handle_cosmos_msg(
            type_urls::MSG_SEND.to_string(),
            Base64VecU8(b"invalid json".to_vec()),
        );

        assert_eq!(response.code, 1);
        assert!(response.log.contains("decode error") || response.log.contains("JSON decode error"));
    }

    #[test]
    fn test_handle_cosmos_msg_empty_amount() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();

        let msg = MsgSend {
            from_address: "alice.near".to_string(),
            to_address: "bob.near".to_string(),
            amount: vec![], // Empty amount
        };

        let msg_data = serde_json::to_vec(&msg).unwrap();
        let msg_data_b64 = Base64VecU8(msg_data);

        let response = contract.handle_cosmos_msg(
            type_urls::MSG_SEND.to_string(),
            msg_data_b64,
        );

        assert_eq!(response.code, 1);
        assert!(response.log.contains("Empty amount"));
    }

    #[test]
    fn test_handle_cosmos_msg_burn() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();

        let msg = MsgBurn {
            from_address: "alice.near".to_string(),
            amount: vec![Coin::new("near", "100000")],
        };

        let msg_data = serde_json::to_vec(&msg).unwrap();
        let msg_data_b64 = Base64VecU8(msg_data);

        let response = contract.handle_cosmos_msg(
            type_urls::MSG_BURN.to_string(),
            msg_data_b64,
        );

        assert_eq!(response.code, 0);
        assert!(response.log.contains("Burned"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "burn");
    }

    #[test]
    fn test_handle_cosmos_msg_multi_send() {
        let context = get_context(accounts(0));
        testing_env!(context.build());

        let mut contract = CosmosContract::new();

        let msg = MsgMultiSend {
            inputs: vec![Input {
                address: "alice.near".to_string(),
                coins: vec![Coin::new("near", "1000000")],
            }],
            outputs: vec![
                Output {
                    address: "bob.near".to_string(),
                    coins: vec![Coin::new("near", "500000")],
                },
                Output {
                    address: "charlie.near".to_string(),
                    coins: vec![Coin::new("near", "500000")],
                },
            ],
        };

        let msg_data = serde_json::to_vec(&msg).unwrap();
        let msg_data_b64 = Base64VecU8(msg_data);

        let response = contract.handle_cosmos_msg(
            type_urls::MSG_MULTI_SEND.to_string(),
            msg_data_b64,
        );

        assert_eq!(response.code, 0);
        assert!(response.log.contains("Multi-send executed"));
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].r#type, "multi_send");
    }
}

#[cfg(test)]
mod lib_tests;