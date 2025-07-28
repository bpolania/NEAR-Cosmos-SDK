use near_sdk::env;
use crate::Balance;

use super::{
    FungibleTokenPacketData, FungibleTokenPacketAcknowledgement, 
    DenomTrace, TransferError, TransferModule
};
use crate::modules::bank::BankModule;
use crate::modules::ibc::channel::{ChannelModule, Packet, Acknowledgement, Height};

/// ICS-20 packet handlers for fungible token transfers
impl TransferModule {
    /// Handle sending a fungible token transfer packet
    /// 
    /// This function is called when a user initiates a cross-chain token transfer.
    /// It validates the transfer, escrows or burns tokens as appropriate, and 
    /// creates the IBC packet.
    /// 
    /// # Arguments
    /// * `channel_module` - Reference to the IBC channel module
    /// * `bank_module` - Mutable reference to the bank module
    /// * `source_port` - Source port identifier (typically "transfer")
    /// * `source_channel` - Source channel identifier
    /// * `token_denom` - Denomination of the token to send
    /// * `amount` - Amount to send
    /// * `sender` - Account sending the tokens
    /// * `receiver` - Destination account address
    /// * `timeout_height` - Packet timeout height
    /// * `timeout_timestamp` - Packet timeout timestamp  
    /// * `memo` - Optional memo string
    /// 
    /// # Returns
    /// * Result containing the packet sequence number
    pub fn send_transfer(
        &mut self,
        channel_module: &mut ChannelModule,
        bank_module: &mut BankModule,
        source_port: String,
        source_channel: String,
        token_denom: String,
        amount: Balance,
        sender: String,
        receiver: String,
        timeout_height: Height,
        timeout_timestamp: u64,
        memo: Option<String>,
    ) -> Result<u64, TransferError> {
        // Validate inputs
        if amount == 0 {
            return Err(TransferError::InvalidAmount);
        }
        
        if receiver.is_empty() {
            return Err(TransferError::InvalidReceiver);
        }

        // Check if channel is open
        if !channel_module.is_channel_open(&source_port, &source_channel) {
            return Err(TransferError::ChannelNotOpen);
        }

        let sender_account = sender.parse()
            .map_err(|_| TransferError::InvalidSender)?;

        // Determine if this is a source zone transfer or not
        let is_source_zone = self.is_source_zone(&source_port, &source_channel, &token_denom);
        
        let packet_denom = if is_source_zone {
            // Token is returning to its source chain - remove the prefix
            self.create_ibc_denom(&source_port, &source_channel, &token_denom)
        } else {
            // Token is being sent away from its source chain - add prefix
            self.create_ibc_denom(&source_port, &source_channel, &token_denom)
        };

        if is_source_zone {
            // Burn voucher tokens (token returning to source)
            self.burn_voucher_tokens(bank_module, &sender, &token_denom, amount)?;
        } else {
            // Escrow native tokens (token leaving source)
            // Check sender balance first
            if bank_module.get_balance(&sender_account) < amount {
                return Err(TransferError::InsufficientFunds);
            }
            
            // Transfer tokens to module account (escrow)
            bank_module.transfer(&sender_account, &env::current_account_id(), amount);
            
            // Track escrowed amount
            self.escrow_tokens(&source_port, &source_channel, &token_denom, amount);
        }

        // Create packet data
        let packet_data = FungibleTokenPacketData::new(
            packet_denom,
            amount.to_string(),
            sender,
            receiver,
            memo,
        );

        // Validate packet data
        packet_data.validate()?;

        // Send packet through channel module
        let sequence = channel_module.send_packet(
            source_port,
            source_channel,
            timeout_height,
            timeout_timestamp,
            packet_data.to_bytes()?,
        ).map_err(|_| TransferError::ChannelNotOpen)?;

        env::log_str(&format!(
            "ICS-20: Sent transfer packet {} for {} {} tokens",
            sequence, amount, token_denom
        ));

        Ok(sequence)
    }

    /// Handle receiving a fungible token transfer packet
    /// 
    /// This function is called when receiving an IBC packet containing a token transfer.
    /// It validates the packet, mints or unescrows tokens as appropriate, and
    /// returns an acknowledgement.
    /// 
    /// # Arguments
    /// * `channel_module` - Reference to the IBC channel module
    /// * `bank_module` - Mutable reference to the bank module
    /// * `packet` - The received IBC packet
    /// 
    /// # Returns
    /// * Result containing the acknowledgement
    pub fn receive_transfer(
        &mut self,
        _channel_module: &ChannelModule,
        bank_module: &mut BankModule,
        packet: &Packet,
    ) -> Result<Acknowledgement, TransferError> {
        // Parse packet data
        let packet_data = FungibleTokenPacketData::from_bytes(&packet.data)
            .map_err(|_| TransferError::InvalidDenomination)?;

        // Validate packet data
        packet_data.validate()?;

        let amount = packet_data.amount_as_balance()?;

        // Determine if this is a source zone for the received token
        let is_source_zone = self.is_source_zone(
            &packet.destination_port,
            &packet.destination_channel,
            &packet_data.denom,
        );

        let result = if is_source_zone {
            // Token is returning to its source - unescrow native tokens
            self.handle_source_zone_receive(
                bank_module,
                &packet.destination_port,
                &packet.destination_channel,
                &packet_data.denom,
                amount,
                &packet_data.receiver,
            )
        } else {
            // Token is arriving from another chain - mint voucher tokens
            self.handle_sink_zone_receive(
                bank_module,
                &packet.destination_port,
                &packet.destination_channel,
                &packet_data.denom,
                amount,
                &packet_data.receiver,
            )
        };

        match result {
            Ok(_) => {
                env::log_str(&format!(
                    "ICS-20: Successfully processed receive for {} {} to {}",
                    amount, packet_data.denom, packet_data.receiver
                ));
                Ok(Acknowledgement::success(FungibleTokenPacketAcknowledgement::success().to_bytes()))
            }
            Err(e) => {
                let error_msg = format!("Transfer failed: {:?}", e);
                env::log_str(&format!("ICS-20: Receive failed: {}", error_msg));
                Ok(Acknowledgement::error(error_msg))
            }
        }
    }

    /// Handle source zone receive (unescrow native tokens)
    fn handle_source_zone_receive(
        &mut self,
        bank_module: &mut BankModule,
        port_id: &str,
        channel_id: &str,
        denom: &str,
        amount: Balance,
        receiver: &str,
    ) -> Result<(), TransferError> {
        // Get the original denomination by removing the prefix
        let original_denom = self.create_ibc_denom(port_id, channel_id, denom);

        // Unescrow tokens
        self.unescrow_tokens(port_id, channel_id, &original_denom, amount)?;

        // Transfer unescrowed tokens to receiver
        let receiver_account = receiver.parse()
            .map_err(|_| TransferError::InvalidReceiver)?;

        bank_module.transfer(&env::current_account_id(), &receiver_account, amount);

        Ok(())
    }

    /// Handle sink zone receive (mint voucher tokens)
    fn handle_sink_zone_receive(
        &mut self,
        bank_module: &mut BankModule,
        port_id: &str,
        channel_id: &str,
        denom: &str,
        amount: Balance,
        receiver: &str,
    ) -> Result<(), TransferError> {
        // Create denomination trace for the received token
        let trace_path = format!("{}/{}/{}", port_id, channel_id, denom);
        let denom_trace = DenomTrace::from_path(&trace_path)?;

        // Register the denomination trace
        let ibc_denom = self.register_denom_trace(denom_trace);

        // Mint voucher tokens to receiver
        self.mint_voucher_tokens(bank_module, receiver, &ibc_denom, amount)?;

        Ok(())
    }



    /// Validate a transfer request before processing
    /// 
    /// This function performs comprehensive validation of transfer parameters
    /// including balance checks, channel state, and denomination validation.
    pub fn validate_transfer(
        &self,
        channel_module: &ChannelModule,
        bank_module: &BankModule,
        source_port: &str,
        source_channel: &str,
        denom: &str,
        amount: Balance,
        sender: &str,
    ) -> Result<(), TransferError> {
        // Basic validation
        if amount == 0 {
            return Err(TransferError::InvalidAmount);
        }

        if denom.is_empty() {
            return Err(TransferError::InvalidDenomination);
        }

        if sender.is_empty() {
            return Err(TransferError::InvalidSender);
        }

        // Check channel state
        if !channel_module.is_channel_open(source_port, source_channel) {
            return Err(TransferError::ChannelNotOpen);
        }

        // Parse and validate sender account
        let sender_account = sender.parse()
            .map_err(|_| TransferError::InvalidSender)?;

        // Check balance for native tokens
        if !self.is_source_zone(source_port, source_channel, denom) {
            if bank_module.get_balance(&sender_account) < amount {
                return Err(TransferError::InsufficientFunds);
            }
        } else {
            // For voucher tokens, check if sufficient supply exists
            let voucher_supply = self.get_voucher_supply(denom);
            if voucher_supply < amount {
                return Err(TransferError::InsufficientVoucherSupply);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::ibc::channel::Order;

    fn create_test_channel_module() -> ChannelModule {
        let mut channel_module = ChannelModule::new();
        
        // Create a test channel in Init state
        channel_module.chan_open_init(
            "transfer".to_string(),
            Order::Unordered,
            vec!["connection-0".to_string()],
            "transfer".to_string(),
            "ics20-1".to_string(),
        );
        
        channel_module
    }

    fn create_test_bank_module() -> BankModule {
        BankModule::new()
    }

    #[test]
    fn test_validate_transfer() {
        let transfer_module = TransferModule::new();
        let channel_module = create_test_channel_module();
        let bank_module = create_test_bank_module();

        // Test channel validation (channel is in Init state, not Open)
        let result = transfer_module.validate_transfer(
            &channel_module,
            &bank_module,
            "transfer",
            "channel-0",
            "unear",
            1000000,
            "alice.near",
        );
        
        // This will fail because channel is not in Open state
        assert!(matches!(result, Err(TransferError::ChannelNotOpen)));

        // Test zero amount
        let result = transfer_module.validate_transfer(
            &channel_module,
            &bank_module,
            "transfer",
            "channel-0",
            "unear",
            0,
            "alice.near",
        );
        
        assert_eq!(result.unwrap_err(), TransferError::InvalidAmount);
    }

    #[test]
    fn test_packet_data_creation() {
        // Test creating and validating packet data
        let packet_data = FungibleTokenPacketData::new(
            "unear".to_string(),
            "1000000".to_string(),
            "alice.near".to_string(),
            "cosmos1abc".to_string(),
            Some("test transfer".to_string()),
        );

        assert_eq!(packet_data.denom, "unear");
        assert_eq!(packet_data.amount, "1000000");
        assert_eq!(packet_data.sender, "alice.near");
        assert_eq!(packet_data.receiver, "cosmos1abc");
        assert_eq!(packet_data.memo, "test transfer");
        
        // Test validation
        assert!(packet_data.validate().is_ok());
        
        // Test serialization
        let bytes = packet_data.to_bytes().unwrap();
        assert!(!bytes.is_empty());
        
        // Test deserialization
        let parsed = FungibleTokenPacketData::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.denom, "unear");
        assert_eq!(parsed.amount, "1000000");
    }

    #[test] 
    fn test_balance_validation() {
        let transfer_module = TransferModule::new();
        let bank_module = create_test_bank_module();

        // Test balance checking for transfers
        let sender_account = "alice.near".parse().unwrap();
        let balance = bank_module.get_balance(&sender_account);
        assert_eq!(balance, 0); // New account should have 0 balance
        
        // Test source zone detection
        assert!(!transfer_module.is_source_zone("transfer", "channel-0", "unear"));
        assert!(transfer_module.is_source_zone("transfer", "channel-0", "transfer/channel-0/uatom"));
    }

    #[test]
    fn test_source_zone_receive_logic() {
        let mut transfer_module = TransferModule::new();
        let mut bank_module = create_test_bank_module();

        // Test handling source zone receive (should fail due to no escrowed tokens)
        let result = transfer_module.handle_source_zone_receive(
            &mut bank_module,
            "transfer",
            "channel-0",
            "transfer/channel-0/unear",
            1000000,
            "alice.near",
        );

        assert!(matches!(result, Err(TransferError::InsufficientEscrow)));
    }

    #[test]
    fn test_sink_zone_receive_logic() {
        let mut transfer_module = TransferModule::new();
        let mut bank_module = create_test_bank_module();

        // Test handling sink zone receive (should mint voucher tokens)
        let result = transfer_module.handle_sink_zone_receive(
            &mut bank_module,
            "transfer",
            "channel-0",
            "uatom",
            1000000,
            "alice.near",
        );

        // This should succeed and create voucher tokens
        assert!(result.is_ok());

        // Check that a voucher supply was created
        // The exact denomination depends on the hash, but we can verify some state was created
        // We can't access private fields, so we just verify the operation succeeded
    }

    #[test]
    fn test_escrow_tracking() {
        let mut transfer_module = TransferModule::new();

        // Test escrow amount tracking
        assert_eq!(transfer_module.get_escrowed_amount("transfer", "channel-0", "unear"), 0);

        // Add tokens to escrow
        transfer_module.escrow_tokens("transfer", "channel-0", "unear", 500000);
        assert_eq!(transfer_module.get_escrowed_amount("transfer", "channel-0", "unear"), 500000);

        // Add more tokens
        transfer_module.escrow_tokens("transfer", "channel-0", "unear", 300000);
        assert_eq!(transfer_module.get_escrowed_amount("transfer", "channel-0", "unear"), 800000);

        // Test unescrow
        let result = transfer_module.unescrow_tokens("transfer", "channel-0", "unear", 200000);
        assert!(result.is_ok());
        assert_eq!(transfer_module.get_escrowed_amount("transfer", "channel-0", "unear"), 600000);

        // Test unescrow more than available
        let result = transfer_module.unescrow_tokens("transfer", "channel-0", "unear", 1000000);
        assert!(matches!(result, Err(TransferError::InsufficientEscrow)));
    }

    #[test]
    fn test_voucher_supply_tracking() {
        let mut transfer_module = TransferModule::new();
        let mut bank_module = create_test_bank_module();

        // Test initial voucher supply
        assert_eq!(transfer_module.get_voucher_supply("ibc/test"), 0);

        // Test minting voucher tokens
        let result = transfer_module.mint_voucher_tokens(
            &mut bank_module,
            "alice.near",
            "ibc/test",
            750000,
        );
        assert!(result.is_ok());
        assert_eq!(transfer_module.get_voucher_supply("ibc/test"), 750000);

        // Test burning voucher tokens (succeeds because minting gave alice tokens)
        let result = transfer_module.burn_voucher_tokens(
            &mut bank_module,
            "alice.near",
            "ibc/test",
            100000,
        );
        // This succeeds because the mint operation gave alice the tokens
        assert!(result.is_ok());
        
        // Verify voucher supply was reduced
        assert_eq!(transfer_module.get_voucher_supply("ibc/test"), 650000);
        
        // Test burning more than available voucher supply
        let result = transfer_module.burn_voucher_tokens(
            &mut bank_module,
            "alice.near",
            "ibc/test",
            700000,
        );
        // This should fail due to insufficient voucher supply
        assert!(matches!(result, Err(TransferError::InsufficientVoucherSupply)));
    }

    #[test]
    fn test_denomination_management() {
        let mut transfer_module = TransferModule::new();

        // Test registering denomination traces
        let trace1 = DenomTrace::new("transfer/channel-0".to_string(), "uatom".to_string());
        let ibc_denom1 = transfer_module.register_denom_trace(trace1.clone());
        assert!(ibc_denom1.starts_with("ibc/"));

        // Test retrieving denomination trace
        let hash1 = &ibc_denom1[4..];
        let retrieved_trace = transfer_module.get_denom_trace(hash1);
        assert!(retrieved_trace.is_some());
        assert_eq!(retrieved_trace.unwrap().base_denom, "uatom");

        // Test getting trace path
        let path = transfer_module.get_trace_path(&ibc_denom1);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), "transfer/channel-0");

        // Test with native token (no path)
        let trace2 = DenomTrace::new("".to_string(), "unear".to_string());
        let ibc_denom2 = transfer_module.register_denom_trace(trace2);
        let hash2 = &ibc_denom2[4..];
        let retrieved_trace2 = transfer_module.get_denom_trace(hash2);
        assert!(retrieved_trace2.is_some());
        let trace2 = retrieved_trace2.unwrap();
        assert_eq!(trace2.base_denom, "unear");
        assert!(trace2.is_native());
    }

    #[test]
    fn test_ibc_denomination_creation() {
        let transfer_module = TransferModule::new();

        // Test creating IBC denomination for outgoing transfer
        let denom1 = transfer_module.create_ibc_denom("transfer", "channel-0", "unear");
        assert_eq!(denom1, "transfer/channel-0/unear");

        // Test creating IBC denomination for returning transfer
        let denom2 = transfer_module.create_ibc_denom("transfer", "channel-0", "transfer/channel-0/uatom");
        assert_eq!(denom2, "uatom");

        // Test with multi-hop transfer
        let denom3 = transfer_module.create_ibc_denom("transfer", "channel-1", "transfer/channel-0/uatom");
        assert_eq!(denom3, "transfer/channel-1/transfer/channel-0/uatom");
    }
}