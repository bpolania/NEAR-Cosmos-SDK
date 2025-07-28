use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::env;

pub mod types;

pub use types::{ChannelEnd, Counterparty, State, Order, Packet, Acknowledgement, Height, PacketCommitment, PacketReceipt};

/// IBC Channel Module
/// 
/// This module implements the ICS-04 Channel specification for packet-based
/// communication over established IBC connections.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ChannelModule {
    /// Mapping from (port_id, channel_id) to ChannelEnd
    channels: LookupMap<String, ChannelEnd>,
    
    /// Packet commitments for sent packets: (port_id, channel_id, sequence) -> PacketCommitment
    packet_commitments: LookupMap<String, PacketCommitment>,
    
    /// Packet receipts for received packets: (port_id, channel_id, sequence) -> PacketReceipt
    packet_receipts: LookupMap<String, PacketReceipt>,
    
    /// Packet acknowledgments: (port_id, channel_id, sequence) -> Acknowledgement
    packet_acknowledgements: LookupMap<String, Acknowledgement>,
    
    /// Next sequence send for each channel: (port_id, channel_id) -> sequence
    next_sequence_send: LookupMap<String, u64>,
    
    /// Next sequence receive for each channel: (port_id, channel_id) -> sequence
    next_sequence_recv: LookupMap<String, u64>,
    
    /// Next sequence acknowledge for each channel: (port_id, channel_id) -> sequence
    next_sequence_ack: LookupMap<String, u64>,
    
    /// Counter for generating unique channel IDs
    next_channel_sequence: u64,
}

impl ChannelModule {
    /// Initialize the IBC Channel module
    pub fn new() -> Self {
        Self {
            channels: LookupMap::new(b"o"),
            packet_commitments: LookupMap::new(b"p"),
            packet_receipts: LookupMap::new(b"q"),
            packet_acknowledgements: LookupMap::new(b"r"),
            next_sequence_send: LookupMap::new(b"s"),
            next_sequence_recv: LookupMap::new(b"t"),
            next_sequence_ack: LookupMap::new(b"u"),
            next_channel_sequence: 0,
        }
    }

    /// Generate a channel key for storage
    fn channel_key(port_id: &str, channel_id: &str) -> String {
        format!("{}#{}", port_id, channel_id)
    }

    /// Generate a packet key for storage
    fn packet_key(port_id: &str, channel_id: &str, sequence: u64) -> String {
        format!("{}#{}#{}", port_id, channel_id, sequence)
    }

    /// Initiate a channel handshake (ChanOpenInit)
    /// 
    /// This function creates a new channel in INIT state.
    /// 
    /// # Arguments
    /// * `port_id` - The port identifier
    /// * `order` - The channel ordering (ORDERED or UNORDERED)
    /// * `connection_hops` - Connection IDs this channel will use
    /// * `counterparty_port_id` - Port ID on the counterparty chain
    /// * `version` - Channel version for application protocol
    /// 
    /// # Returns
    /// * The generated channel ID
    pub fn chan_open_init(
        &mut self,
        port_id: String,
        order: Order,
        connection_hops: Vec<String>,
        counterparty_port_id: String,
        version: String,
    ) -> String {
        // Generate unique channel ID
        let channel_id = format!("channel-{}", self.next_channel_sequence);
        self.next_channel_sequence += 1;

        // Create counterparty without channel ID (will be set later)
        let counterparty = Counterparty::new(counterparty_port_id, None);

        // Create channel end in INIT state
        let channel_end = ChannelEnd::new(
            State::Init,
            order,
            counterparty,
            connection_hops,
            version,
        );

        // Store the channel
        let key = Self::channel_key(&port_id, &channel_id);
        self.channels.insert(&key, &channel_end);

        // Initialize sequence numbers
        let seq_key = Self::channel_key(&port_id, &channel_id);
        self.next_sequence_send.insert(&seq_key, &1);
        self.next_sequence_recv.insert(&seq_key, &1);
        self.next_sequence_ack.insert(&seq_key, &1);

        env::log_str(&format!(
            "Channel: Initiated channel {} on port {} in INIT state",
            channel_id, port_id
        ));

        channel_id
    }

    /// Respond to a channel handshake (ChanOpenTry)
    /// 
    /// This function creates a new channel in TRYOPEN state in response to
    /// a ChanOpenInit from the counterparty.
    /// 
    /// # Arguments
    /// * `port_id` - The port identifier
    /// * `previous_channel_id` - The channel ID (if retrying)
    /// * `order` - The channel ordering
    /// * `connection_hops` - Connection IDs this channel will use
    /// * `counterparty_port_id` - Port ID on the counterparty chain
    /// * `counterparty_channel_id` - Channel ID on the counterparty chain
    /// * `version` - Channel version
    /// * `counterparty_version` - Version proposed by counterparty
    /// * `channel_proof` - Proof of channel on counterparty
    /// * `proof_height` - Height at which proof was generated
    /// 
    /// # Returns
    /// * The channel ID
    pub fn chan_open_try(
        &mut self,
        port_id: String,
        previous_channel_id: Option<String>,
        order: Order,
        connection_hops: Vec<String>,
        counterparty_port_id: String,
        counterparty_channel_id: String,
        version: String,
        _counterparty_version: String,
        _channel_proof: Vec<u8>,
        _proof_height: u64,
    ) -> Result<String, String> {
        // Verify channel proof using connection module
        self.verify_channel_try_proof(
            &port_id,
            &counterparty_channel_id,
            &_channel_proof,
            _proof_height,
        )?;

        let is_new_channel = previous_channel_id.is_none();
        let channel_id = if let Some(chan_id) = previous_channel_id {
            // Reuse existing channel ID
            chan_id
        } else {
            // Generate new channel ID
            let chan_id = format!("channel-{}", self.next_channel_sequence);
            self.next_channel_sequence += 1;
            chan_id
        };

        // Create counterparty with channel ID
        let counterparty = Counterparty::new(counterparty_port_id, Some(counterparty_channel_id));

        // Create channel end in TRYOPEN state
        let channel_end = ChannelEnd::new(
            State::TryOpen,
            order,
            counterparty,
            connection_hops,
            version,
        );

        // Store the channel
        let key = Self::channel_key(&port_id, &channel_id);
        self.channels.insert(&key, &channel_end);

        // Initialize sequence numbers if new channel
        if is_new_channel {
            let seq_key = Self::channel_key(&port_id, &channel_id);
            self.next_sequence_send.insert(&seq_key, &1);
            self.next_sequence_recv.insert(&seq_key, &1);
            self.next_sequence_ack.insert(&seq_key, &1);
        }

        env::log_str(&format!(
            "Channel: Created channel {} on port {} in TRYOPEN state",
            channel_id, port_id
        ));

        Ok(channel_id)
    }

    /// Acknowledge a channel handshake (ChanOpenAck)
    /// 
    /// This function moves a channel from INIT to OPEN state upon receiving
    /// acknowledgment from the counterparty.
    /// 
    /// # Arguments
    /// * `port_id` - The port identifier
    /// * `channel_id` - The channel ID to acknowledge
    /// * `counterparty_channel_id` - Channel ID on counterparty chain
    /// * `counterparty_version` - Version confirmed by counterparty
    /// * `channel_proof` - Proof of channel on counterparty
    /// * `proof_height` - Height at which proof was generated
    /// 
    /// # Returns
    /// * Success or failure
    pub fn chan_open_ack(
        &mut self,
        port_id: String,
        channel_id: String,
        counterparty_channel_id: String,
        _counterparty_version: String,
        _channel_proof: Vec<u8>,
        _proof_height: u64,
    ) -> Result<(), String> {
        // Get the existing channel
        let key = Self::channel_key(&port_id, &channel_id);
        let mut channel = self.channels.get(&key)
            .ok_or("Channel not found")?;

        // Verify channel is in INIT state
        if channel.state != State::Init {
            return Err("Channel not in INIT state".to_string());
        }

        // Verify channel proof using connection module
        self.verify_channel_ack_proof(
            &port_id,
            &channel_id,
            &counterparty_channel_id,
            &_channel_proof,
            _proof_height,
        )?;

        // Update channel to OPEN state
        channel.state = State::Open;
        channel.counterparty.channel_id = Some(counterparty_channel_id.clone());

        // Store updated channel
        self.channels.insert(&key, &channel);

        env::log_str(&format!(
            "Channel: Acknowledged channel {} on port {} - now OPEN with counterparty {}",
            channel_id, port_id, counterparty_channel_id
        ));

        Ok(())
    }

    /// Confirm a channel handshake (ChanOpenConfirm)
    /// 
    /// This function moves a channel from TRYOPEN to OPEN state upon
    /// confirmation from the counterparty.
    /// 
    /// # Arguments
    /// * `port_id` - The port identifier
    /// * `channel_id` - The channel ID to confirm
    /// * `channel_proof` - Proof of channel on counterparty
    /// * `proof_height` - Height at which proof was generated
    /// 
    /// # Returns
    /// * Success or failure
    pub fn chan_open_confirm(
        &mut self,
        port_id: String,
        channel_id: String,
        _channel_proof: Vec<u8>,
        _proof_height: u64,
    ) -> Result<(), String> {
        // Get the existing channel
        let key = Self::channel_key(&port_id, &channel_id);
        let mut channel = self.channels.get(&key)
            .ok_or("Channel not found")?;

        // Verify channel is in TRYOPEN state
        if channel.state != State::TryOpen {
            return Err("Channel not in TRYOPEN state".to_string());
        }

        // Verify channel proof using connection module
        self.verify_channel_confirm_proof(
            &port_id,
            &channel_id,
            &_channel_proof,
            _proof_height,
        )?;

        // Update channel to OPEN state
        channel.state = State::Open;

        // Store updated channel
        self.channels.insert(&key, &channel);

        env::log_str(&format!(
            "Channel: Confirmed channel {} on port {} - now OPEN",
            channel_id, port_id
        ));

        Ok(())
    }

    /// Send a packet over an IBC channel
    /// 
    /// # Arguments
    /// * `source_port` - Source port identifier
    /// * `source_channel` - Source channel identifier
    /// * `timeout_height` - Packet timeout height
    /// * `timeout_timestamp` - Packet timeout timestamp
    /// * `data` - Packet data payload
    /// 
    /// # Returns
    /// * The packet sequence number
    pub fn send_packet(
        &mut self,
        source_port: String,
        source_channel: String,
        timeout_height: Height,
        timeout_timestamp: u64,
        data: Vec<u8>,
    ) -> Result<u64, String> {
        // Get the channel
        let key = Self::channel_key(&source_port, &source_channel);
        let channel = self.channels.get(&key)
            .ok_or("Channel not found")?;

        // Verify channel is open
        if !channel.is_open() {
            return Err("Channel is not open".to_string());
        }

        // Get next sequence number
        let sequence = self.next_sequence_send.get(&key).unwrap_or(1);

        // Get destination info from channel
        let dest_port = channel.counterparty.port_id.clone();
        let dest_channel = channel.counterparty.channel_id
            .clone()
            .ok_or("Counterparty channel ID not set")?;

        // Create packet
        let packet = Packet::new(
            sequence,
            source_port.clone(),
            source_channel.clone(),
            dest_port,
            dest_channel,
            data,
            timeout_height,
            timeout_timestamp,
        );

        // Store packet commitment
        let packet_key = Self::packet_key(&source_port, &source_channel, sequence);
        let commitment = PacketCommitment::from_packet(&packet);
        self.packet_commitments.insert(&packet_key, &commitment);

        // Increment sequence number
        self.next_sequence_send.insert(&key, &(sequence + 1));

        env::log_str(&format!(
            "Packet: Sent packet {} on channel {}:{} with commitment",
            sequence, source_port, source_channel
        ));

        Ok(sequence)
    }

    /// Receive a packet from an IBC channel
    /// 
    /// # Arguments
    /// * `packet` - The packet to receive
    /// * `packet_proof` - Proof of packet on sending chain
    /// * `proof_height` - Height at which proof was generated
    /// 
    /// # Returns
    /// * Success or failure
    pub fn recv_packet(
        &mut self,
        packet: Packet,
        _packet_proof: Vec<u8>,
        _proof_height: u64,
    ) -> Result<(), String> {
        // Get the channel
        let key = Self::channel_key(&packet.destination_port, &packet.destination_channel);
        let channel = self.channels.get(&key)
            .ok_or("Channel not found")?;

        // Verify channel is open
        if !channel.is_open() {
            return Err("Channel is not open".to_string());
        }

        // Check if packet has already been received
        let packet_key = Self::packet_key(&packet.destination_port, &packet.destination_channel, packet.sequence);
        if self.packet_receipts.contains_key(&packet_key) {
            return Err("Packet already received".to_string());
        }

        // Verify packet timeout
        let current_height = Height::new(1, env::block_height()); // Simplified height
        let current_timestamp = env::block_timestamp();
        
        if packet.is_timed_out_on_height(&current_height) {
            return Err("Packet timed out on height".to_string());
        }
        
        if packet.is_timed_out_on_timestamp(current_timestamp) {
            return Err("Packet timed out on timestamp".to_string());
        }

        // For ordered channels, verify sequence
        if channel.ordering == Order::Ordered {
            let expected_sequence = self.next_sequence_recv.get(&key).unwrap_or(1);
            if packet.sequence != expected_sequence {
                return Err(format!("Expected sequence {}, got {}", expected_sequence, packet.sequence));
            }
        }

        // Verify packet proof using connection module
        self.verify_packet_commitment_proof(
            &packet,
            &_packet_proof,
            _proof_height,
        )?;

        // Store packet receipt
        let receipt = PacketReceipt::new(packet.sequence);
        self.packet_receipts.insert(&packet_key, &receipt);

        // Update next receive sequence for ordered channels
        if channel.ordering == Order::Ordered {
            let next_seq = self.next_sequence_recv.get(&key).unwrap_or(1);
            self.next_sequence_recv.insert(&key, &(next_seq + 1));
        }

        env::log_str(&format!(
            "Packet: Received packet {} on channel {}:{}",
            packet.sequence, packet.destination_port, packet.destination_channel
        ));

        Ok(())
    }

    /// Acknowledge a packet
    /// 
    /// # Arguments
    /// * `packet` - The original packet
    /// * `acknowledgement` - The acknowledgement data
    /// * `ack_proof` - Proof of acknowledgement on receiving chain
    /// * `proof_height` - Height at which proof was generated
    /// 
    /// # Returns
    /// * Success or failure
    pub fn acknowledge_packet(
        &mut self,
        packet: Packet,
        acknowledgement: Acknowledgement,
        _ack_proof: Vec<u8>,
        _proof_height: u64,
    ) -> Result<(), String> {
        // Get the channel
        let key = Self::channel_key(&packet.source_port, &packet.source_channel);
        let channel = self.channels.get(&key)
            .ok_or("Channel not found")?;

        // Verify channel is open
        if !channel.is_open() {
            return Err("Channel is not open".to_string());
        }

        // Check if packet commitment exists
        let packet_key = Self::packet_key(&packet.source_port, &packet.source_channel, packet.sequence);
        if !self.packet_commitments.contains_key(&packet_key) {
            return Err("Packet commitment not found".to_string());
        }

        // Check if already acknowledged
        if self.packet_acknowledgements.contains_key(&packet_key) {
            return Err("Packet already acknowledged".to_string());
        }

        // For ordered channels, verify sequence
        if channel.ordering == Order::Ordered {
            let expected_sequence = self.next_sequence_ack.get(&key).unwrap_or(1);
            if packet.sequence != expected_sequence {
                return Err(format!("Expected ack sequence {}, got {}", expected_sequence, packet.sequence));
            }
        }

        // Verify acknowledgement proof using connection module
        self.verify_packet_acknowledgement_proof(
            &packet,
            &acknowledgement,
            &_ack_proof,
            _proof_height,
        )?;

        // Store acknowledgement
        self.packet_acknowledgements.insert(&packet_key, &acknowledgement);

        // Remove packet commitment
        self.packet_commitments.remove(&packet_key);

        // Update next ack sequence for ordered channels
        if channel.ordering == Order::Ordered {
            let next_seq = self.next_sequence_ack.get(&key).unwrap_or(1);
            self.next_sequence_ack.insert(&key, &(next_seq + 1));
        }

        env::log_str(&format!(
            "Packet: Acknowledged packet {} on channel {}:{}",
            packet.sequence, packet.source_port, packet.source_channel
        ));

        Ok(())
    }

    /// Get a channel by port and channel ID
    pub fn get_channel(&self, port_id: String, channel_id: String) -> Option<ChannelEnd> {
        let key = Self::channel_key(&port_id, &channel_id);
        self.channels.get(&key)
    }

    /// Check if a channel exists and is open
    pub fn is_channel_open(&self, port_id: &str, channel_id: &str) -> bool {
        let key = Self::channel_key(port_id, channel_id);
        self.channels.get(&key)
            .map(|channel| channel.is_open())
            .unwrap_or(false)
    }

    /// Get next sequence send for a channel
    pub fn get_next_sequence_send(&self, port_id: &str, channel_id: &str) -> u64 {
        let key = Self::channel_key(port_id, channel_id);
        self.next_sequence_send.get(&key).unwrap_or(1)
    }

    /// Get next sequence receive for a channel
    pub fn get_next_sequence_recv(&self, port_id: &str, channel_id: &str) -> u64 {
        let key = Self::channel_key(port_id, channel_id);
        self.next_sequence_recv.get(&key).unwrap_or(1)
    }

    /// Get packet commitment
    pub fn get_packet_commitment(&self, port_id: &str, channel_id: &str, sequence: u64) -> Option<PacketCommitment> {
        let key = Self::packet_key(port_id, channel_id, sequence);
        self.packet_commitments.get(&key)
    }

    /// Get packet receipt
    pub fn get_packet_receipt(&self, port_id: &str, channel_id: &str, sequence: u64) -> Option<PacketReceipt> {
        let key = Self::packet_key(port_id, channel_id, sequence);
        self.packet_receipts.get(&key)
    }

    /// Get packet acknowledgement
    pub fn get_packet_acknowledgement(&self, port_id: &str, channel_id: &str, sequence: u64) -> Option<Acknowledgement> {
        let key = Self::packet_key(port_id, channel_id, sequence);
        self.packet_acknowledgements.get(&key)
    }

    /// Create a success acknowledgement for a packet
    pub fn create_success_acknowledgement(&self, result: Vec<u8>) -> Acknowledgement {
        Acknowledgement::success(result)
    }

    /// Create an error acknowledgement for a packet
    pub fn create_error_acknowledgement(&self, error: String) -> Acknowledgement {
        Acknowledgement::error(error)
    }

    /// Check if an acknowledgement represents success
    pub fn is_acknowledgement_success(&self, ack: &Acknowledgement) -> bool {
        ack.is_success()
    }

    /// Create a packet commitment from raw data
    pub fn create_packet_commitment(&self, data: Vec<u8>) -> PacketCommitment {
        PacketCommitment::new(data)
    }

    /// Check if a timeout height is zero (no height timeout)
    pub fn is_timeout_height_zero(&self, height: &types::Height) -> bool {
        height.is_zero()
    }

    // Proof verification methods (simplified for now)

    fn verify_channel_try_proof(
        &self,
        _port_id: &str,
        _counterparty_channel_id: &str,
        channel_proof: &[u8],
        _proof_height: u64,
    ) -> Result<(), String> {
        if channel_proof.is_empty() {
            return Err("Channel proof cannot be empty".to_string());
        }
        Ok(())
    }

    fn verify_channel_ack_proof(
        &self,
        _port_id: &str,
        _channel_id: &str,
        _counterparty_channel_id: &str,
        channel_proof: &[u8],
        _proof_height: u64,
    ) -> Result<(), String> {
        if channel_proof.is_empty() {
            return Err("Channel proof cannot be empty".to_string());
        }
        Ok(())
    }

    fn verify_channel_confirm_proof(
        &self,
        _port_id: &str,
        _channel_id: &str,
        channel_proof: &[u8],
        _proof_height: u64,
    ) -> Result<(), String> {
        if channel_proof.is_empty() {
            return Err("Channel proof cannot be empty".to_string());
        }
        Ok(())
    }

    fn verify_packet_commitment_proof(
        &self,
        _packet: &Packet,
        packet_proof: &[u8],
        _proof_height: u64,
    ) -> Result<(), String> {
        if packet_proof.is_empty() {
            return Err("Packet proof cannot be empty".to_string());
        }
        Ok(())
    }

    fn verify_packet_acknowledgement_proof(
        &self,
        _packet: &Packet,
        _acknowledgement: &Acknowledgement,
        ack_proof: &[u8],
        _proof_height: u64,
    ) -> Result<(), String> {
        if ack_proof.is_empty() {
            return Err("Acknowledgement proof cannot be empty".to_string());
        }
        Ok(())
    }
}