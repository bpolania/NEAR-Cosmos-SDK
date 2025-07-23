use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::env;
use crate::Balance;

pub mod types;
pub mod handlers;

pub use types::{
    FungibleTokenPacketData, DenomTrace,
    FungibleTokenPacketAcknowledgement, TransferError
};

use crate::modules::bank::BankModule;

/// ICS-20 Fungible Token Transfer Module
/// 
/// This module implements the ICS-20 specification for cross-chain fungible token transfers.
/// It handles token escrow/mint mechanics, denomination tracing, and integration with
/// the existing IBC channel infrastructure.
#[derive(BorshDeserialize, BorshSerialize)]
pub struct TransferModule {
    /// Mapping from denomination trace hash to DenomTrace
    /// Key: hash(trace_path), Value: DenomTrace
    denom_traces: LookupMap<String, DenomTrace>,
    
    /// Mapping from IBC denomination to native denomination
    /// Key: ibc/hash, Value: trace_path
    denom_to_trace: LookupMap<String, String>,
    
    /// Escrowed tokens for each channel: (port_id, channel_id, denom) -> amount
    escrowed_tokens: LookupMap<String, Balance>,
    
    /// Total supply of voucher tokens: denom -> amount
    voucher_supply: LookupMap<String, Balance>,
    
    /// Port ID for this transfer module (typically "transfer")
    port_id: String,
}

impl TransferModule {
    /// Initialize the ICS-20 Transfer module
    pub fn new() -> Self {
        Self {
            denom_traces: LookupMap::new(b"a"),
            denom_to_trace: LookupMap::new(b"b"),
            escrowed_tokens: LookupMap::new(b"c"),
            voucher_supply: LookupMap::new(b"d"),
            port_id: "transfer".to_string(),
        }
    }


    /// Generate escrow key for token storage
    fn escrow_key(port_id: &str, channel_id: &str, denom: &str) -> String {
        format!("{}#{}#{}", port_id, channel_id, denom)
    }

    /// Get escrowed token amount
    pub fn get_escrowed_amount(&self, port_id: &str, channel_id: &str, denom: &str) -> Balance {
        let key = Self::escrow_key(port_id, channel_id, denom);
        self.escrowed_tokens.get(&key).unwrap_or(0)
    }

    /// Add tokens to escrow
    fn escrow_tokens(&mut self, port_id: &str, channel_id: &str, denom: &str, amount: Balance) {
        let key = Self::escrow_key(port_id, channel_id, denom);
        let current = self.escrowed_tokens.get(&key).unwrap_or(0);
        self.escrowed_tokens.insert(&key, &(current + amount));
        
        env::log_str(&format!(
            "Escrowed {} {} on channel {}",
            amount, denom, channel_id
        ));
    }

    /// Remove tokens from escrow
    fn unescrow_tokens(&mut self, port_id: &str, channel_id: &str, denom: &str, amount: Balance) -> Result<(), TransferError> {
        let key = Self::escrow_key(port_id, channel_id, denom);
        let current = self.escrowed_tokens.get(&key).unwrap_or(0);
        
        if current < amount {
            return Err(TransferError::InsufficientEscrow);
        }
        
        self.escrowed_tokens.insert(&key, &(current - amount));
        
        env::log_str(&format!(
            "Unescrowed {} {} from channel {}",
            amount, denom, channel_id
        ));
        
        Ok(())
    }

    /// Register a new denomination trace
    pub fn register_denom_trace(&mut self, trace: DenomTrace) -> String {
        let trace_hash = trace.hash();
        let ibc_denom = format!("ibc/{}", trace_hash);
        
        self.denom_traces.insert(&trace_hash, &trace);
        self.denom_to_trace.insert(&ibc_denom, &trace.path);
        
        env::log_str(&format!(
            "Registered denomination trace: {} -> {}",
            ibc_denom, trace.path
        ));
        
        ibc_denom
    }

    /// Get denomination trace by hash
    pub fn get_denom_trace(&self, trace_hash: &str) -> Option<DenomTrace> {
        self.denom_traces.get(&trace_hash.to_string())
    }

    /// Get trace path by IBC denomination
    pub fn get_trace_path(&self, ibc_denom: &str) -> Option<String> {
        self.denom_to_trace.get(&ibc_denom.to_string())
    }

    /// Check if a denomination is from this chain (source zone)
    pub fn is_source_zone(&self, port_id: &str, channel_id: &str, denom: &str) -> bool {
        // If denom starts with port/channel prefix, it's returning to source
        let prefix = format!("{}/{}/", port_id, channel_id);
        denom.starts_with(&prefix)
    }

    /// Create IBC denomination for a token being sent out
    pub fn create_ibc_denom(&self, port_id: &str, channel_id: &str, denom: &str) -> String {
        if self.is_source_zone(port_id, channel_id, denom) {
            // Remove the prefix to get original denomination
            let prefix = format!("{}/{}/", port_id, channel_id);
            denom.strip_prefix(&prefix).unwrap_or(denom).to_string()
        } else {
            // Add prefix for cross-chain denomination
            format!("{}/{}/{}", port_id, channel_id, denom)
        }
    }

    /// Mint voucher tokens (when receiving from another chain)
    pub fn mint_voucher_tokens(
        &mut self, 
        bank_module: &mut BankModule,
        receiver: &str,
        denom: &str,
        amount: Balance,
    ) -> Result<(), TransferError> {
        // Update voucher supply tracking
        let current_supply = self.voucher_supply.get(&denom.to_string()).unwrap_or(0);
        self.voucher_supply.insert(&denom.to_string(), &(current_supply + amount));
        
        // Mint tokens through bank module
        let receiver_account = receiver.parse()
            .map_err(|_| TransferError::InvalidReceiver)?;
        
        bank_module.mint(&receiver_account, amount);
        
        env::log_str(&format!(
            "Minted {} voucher tokens {} to {}",
            amount, denom, receiver
        ));
        
        Ok(())
    }

    /// Burn voucher tokens (when sending back to source chain)
    pub fn burn_voucher_tokens(
        &mut self,
        bank_module: &mut BankModule, 
        sender: &str,
        denom: &str,
        amount: Balance,
    ) -> Result<(), TransferError> {
        // Check voucher supply
        let current_supply = self.voucher_supply.get(&denom.to_string()).unwrap_or(0);
        if current_supply < amount {
            return Err(TransferError::InsufficientVoucherSupply);
        }
        
        // Check sender balance through bank module
        let sender_account = sender.parse()
            .map_err(|_| TransferError::InvalidSender)?;
        
        if bank_module.get_balance(&sender_account) < amount {
            return Err(TransferError::InsufficientFunds);
        }
        
        // Burn tokens (transfer to module account)
        bank_module.transfer(&sender_account, &env::current_account_id(), amount);
        
        // Update voucher supply
        self.voucher_supply.insert(&denom.to_string(), &(current_supply - amount));
        
        env::log_str(&format!(
            "Burned {} voucher tokens {} from {}",
            amount, denom, sender
        ));
        
        Ok(())
    }

    /// Get total voucher supply for a denomination
    pub fn get_voucher_supply(&self, denom: &str) -> Balance {
        self.voucher_supply.get(&denom.to_string()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escrow_key_generation() {
        let key = TransferModule::escrow_key("transfer", "channel-0", "uatom");
        assert_eq!(key, "transfer#channel-0#uatom");
    }

    #[test]
    fn test_source_zone_detection() {
        let module = TransferModule::new();
        
        // Native token (not from this chain)
        assert!(!module.is_source_zone("transfer", "channel-0", "unear"));
        
        // Token returning to source (has prefix)
        assert!(module.is_source_zone("transfer", "channel-0", "transfer/channel-0/uatom"));
    }

    #[test]
    fn test_ibc_denom_creation() {
        let module = TransferModule::new();
        
        // Native token going out
        let ibc_denom = module.create_ibc_denom("transfer", "channel-0", "unear");
        assert_eq!(ibc_denom, "transfer/channel-0/unear");
        
        // Token returning to source
        let original_denom = module.create_ibc_denom("transfer", "channel-0", "transfer/channel-0/uatom");
        assert_eq!(original_denom, "uatom");
    }
}