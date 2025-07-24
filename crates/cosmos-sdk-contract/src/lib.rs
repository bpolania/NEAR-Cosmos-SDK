use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

pub type Balance = u128;

mod modules;

use modules::bank::BankModule;
use modules::gov::GovernanceModule;
use modules::staking::StakingModule;
use modules::ibc::client::tendermint::{TendermintLightClientModule, Header, Height};
use modules::ibc::connection::{ConnectionModule, ConnectionEnd, Counterparty, Version};
use modules::ibc::connection::types::{MerklePrefix};
use modules::ibc::channel::{ChannelModule, ChannelEnd, Order, Packet, Acknowledgement};
use modules::ibc::channel::types::{PacketCommitment, PacketReceipt};
use modules::ibc::transfer::{TransferModule, FungibleTokenPacketData, DenomTrace};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosContract {
    bank_module: BankModule,
    staking_module: StakingModule,
    governance_module: GovernanceModule,
    ibc_client_module: TendermintLightClientModule,
    ibc_connection_module: ConnectionModule,
    ibc_channel_module: ChannelModule,
    ibc_transfer_module: TransferModule,
    block_height: u64,
}

#[near_bindgen]
impl CosmosContract {
    #[init]
    pub fn new() -> Self {
        Self {
            bank_module: BankModule::new(),
            staking_module: StakingModule::new(),
            governance_module: GovernanceModule::new(),
            ibc_client_module: TendermintLightClientModule::new(),
            ibc_connection_module: ConnectionModule::new(),
            ibc_channel_module: ChannelModule::new(),
            ibc_transfer_module: TransferModule::new(),
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

    // View functions
    pub fn get_block_height(&self) -> u64 {
        self.block_height
    }
}