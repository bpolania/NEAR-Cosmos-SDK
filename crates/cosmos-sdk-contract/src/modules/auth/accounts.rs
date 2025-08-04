use crate::crypto::CosmosPublicKey;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::AccountId;
use near_sdk::collections::{LookupMap, Vector};
use std::collections::HashMap;

/// Account management errors
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AccountError {
    /// Account not found
    AccountNotFound(String),
    /// Invalid address format
    InvalidAddress(String),
    /// Sequence number mismatch
    SequenceMismatch { expected: u64, actual: u64 },
    /// Account already exists
    AccountExists(String),
    /// Invalid public key
    InvalidPublicKey(String),
    /// Address derivation failed
    AddressDerivationFailed(String),
}

impl std::fmt::Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountError::AccountNotFound(addr) => write!(f, "Account not found: {}", addr),
            AccountError::InvalidAddress(addr) => write!(f, "Invalid address format: {}", addr),
            AccountError::SequenceMismatch { expected, actual } => {
                write!(f, "Sequence mismatch: expected {}, got {}", expected, actual)
            }
            AccountError::AccountExists(addr) => write!(f, "Account already exists: {}", addr),
            AccountError::InvalidPublicKey(msg) => write!(f, "Invalid public key: {}", msg),
            AccountError::AddressDerivationFailed(msg) => write!(f, "Address derivation failed: {}", msg),
        }
    }
}

impl std::error::Error for AccountError {}

/// Cosmos-compatible account structure
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct CosmosAccount {
    /// Cosmos bech32 address
    pub address: String,
    /// Sequential account number (unique, monotonic)
    pub account_number: u64,
    /// Transaction sequence number (for replay protection)
    pub sequence: u64,
    /// Public key (optional, set after first transaction)
    pub public_key: Option<CosmosPublicKey>,
    /// NEAR account ID (for compatibility)
    pub near_account_id: Option<AccountId>,
}

impl CosmosAccount {
    /// Create a new account
    pub fn new(address: String, account_number: u64) -> Self {
        Self {
            address,
            account_number,
            sequence: 0,
            public_key: None,
            near_account_id: None,
        }
    }

    /// Set the public key
    pub fn set_public_key(&mut self, public_key: CosmosPublicKey) {
        self.public_key = Some(public_key);
    }

    /// Set the NEAR account ID for compatibility
    pub fn set_near_account_id(&mut self, account_id: AccountId) {
        self.near_account_id = Some(account_id);
    }

    /// Increment the sequence number
    pub fn increment_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    /// Validate the sequence number
    pub fn validate_sequence(&self, expected_sequence: u64) -> Result<(), AccountError> {
        if self.sequence != expected_sequence {
            return Err(AccountError::SequenceMismatch {
                expected: self.sequence,
                actual: expected_sequence,
            });
        }
        Ok(())
    }

    /// Check if account has a public key set
    pub fn has_public_key(&self) -> bool {
        self.public_key.is_some()
    }
}

/// Account manager for Cosmos SDK compatibility
#[derive(BorshSerialize, BorshDeserialize)]
pub struct AccountManager {
    /// Map from address to account info
    accounts: LookupMap<String, CosmosAccount>,
    /// Map from NEAR account ID to Cosmos address
    near_to_cosmos: LookupMap<AccountId, String>,
    /// Vector of account addresses for listing (since LookupMap doesn't support iteration)
    account_addresses: Vector<String>,
    /// Next account number to assign
    next_account_number: u64,
    /// Configuration
    config: AccountConfig,
}

/// Account manager configuration
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct AccountConfig {
    /// Address prefix for bech32 encoding (e.g., "cosmos", "near")
    pub address_prefix: String,
    /// Enable automatic account creation
    pub auto_create_accounts: bool,
    /// Maximum sequence number allowed (for safety)
    pub max_sequence: u64,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            address_prefix: "near".to_string(),
            auto_create_accounts: true,
            max_sequence: 1_000_000,
        }
    }
}

impl AccountManager {
    /// Create a new account manager
    pub fn new(config: AccountConfig) -> Self {
        Self {
            accounts: LookupMap::new(b"a"),
            near_to_cosmos: LookupMap::new(b"n"),
            account_addresses: Vector::new(b"d"),
            next_account_number: 1, // Start at 1 per Cosmos convention
            config,
        }
    }

    /// Get account by Cosmos address
    pub fn get_account(&self, address: &str) -> Option<CosmosAccount> {
        self.accounts.get(&address.to_string())
    }

    /// Get account by NEAR account ID
    pub fn get_account_by_near_id(&self, near_account_id: &AccountId) -> Option<CosmosAccount> {
        if let Some(cosmos_address) = self.near_to_cosmos.get(near_account_id) {
            self.accounts.get(&cosmos_address)
        } else {
            None
        }
    }

    /// Create a new account with a public key
    pub fn create_account(&mut self, public_key: CosmosPublicKey) -> Result<CosmosAccount, AccountError> {
        // Derive address from public key
        let address = public_key.to_cosmos_address(&self.config.address_prefix)
            .map_err(|e| AccountError::AddressDerivationFailed(e.to_string()))?;

        // Check if account already exists
        if self.accounts.get(&address).is_some() {
            return Err(AccountError::AccountExists(address));
        }

        // Create new account
        let account_number = self.next_account_number;
        self.next_account_number += 1;

        let mut account = CosmosAccount::new(address.clone(), account_number);
        account.set_public_key(public_key);

        // Store the account
        self.accounts.insert(&address, &account);
        self.account_addresses.push(&address);

        Ok(account)
    }

    /// Create account from NEAR account ID (for compatibility)
    pub fn create_account_from_near_id(&mut self, near_account_id: AccountId) -> Result<CosmosAccount, AccountError> {
        // Check if account already exists
        if self.near_to_cosmos.get(&near_account_id).is_some() {
            return Err(AccountError::AccountExists(near_account_id.to_string()));
        }

        // Create a pseudo-address for NEAR accounts
        let address = format!("{}1{}", self.config.address_prefix, near_account_id.as_str().replace(".", ""));

        // Create new account
        let account_number = self.next_account_number;
        self.next_account_number += 1;

        let mut account = CosmosAccount::new(address.clone(), account_number);
        account.set_near_account_id(near_account_id.clone());

        // Store the account and mapping
        self.accounts.insert(&address, &account);
        self.near_to_cosmos.insert(&near_account_id, &address);
        self.account_addresses.push(&address);

        Ok(account)
    }

    /// Update account with public key (for first transaction)
    pub fn set_account_public_key(&mut self, address: &str, public_key: CosmosPublicKey) -> Result<(), AccountError> {
        let mut account = self.accounts.get(&address.to_string())
            .ok_or_else(|| AccountError::AccountNotFound(address.to_string()))?;

        account.set_public_key(public_key);
        self.accounts.insert(&address.to_string(), &account);

        Ok(())
    }

    /// Increment account sequence number
    pub fn increment_sequence(&mut self, address: &str) -> Result<u64, AccountError> {
        let mut account = self.accounts.get(&address.to_string())
            .ok_or_else(|| AccountError::AccountNotFound(address.to_string()))?;

        let new_sequence = account.increment_sequence();

        // Safety check
        if new_sequence > self.config.max_sequence {
            return Err(AccountError::SequenceMismatch {
                expected: account.sequence - 1,
                actual: new_sequence,
            });
        }

        self.accounts.insert(&address.to_string(), &account);
        Ok(new_sequence)
    }

    /// Validate sequence number for replay protection
    pub fn validate_sequence(&self, address: &str, expected_sequence: u64) -> Result<(), AccountError> {
        let account = self.accounts.get(&address.to_string())
            .ok_or_else(|| AccountError::AccountNotFound(address.to_string()))?;

        account.validate_sequence(expected_sequence)
    }

    /// Get or create account (if auto-creation is enabled)
    pub fn get_or_create_account(&mut self, public_key: CosmosPublicKey) -> Result<CosmosAccount, AccountError> {
        let address = public_key.to_cosmos_address(&self.config.address_prefix)
            .map_err(|e| AccountError::AddressDerivationFailed(e.to_string()))?;

        if let Some(account) = self.accounts.get(&address) {
            Ok(account)
        } else if self.config.auto_create_accounts {
            self.create_account(public_key)
        } else {
            Err(AccountError::AccountNotFound(address))
        }
    }

    /// Get account numbers for a list of addresses
    pub fn get_account_numbers(&self, addresses: &[String]) -> Result<Vec<u64>, AccountError> {
        let mut account_numbers = Vec::new();

        for address in addresses {
            let account = self.accounts.get(&address.to_string())
                .ok_or_else(|| AccountError::AccountNotFound(address.to_string()))?;
            account_numbers.push(account.account_number);
        }

        Ok(account_numbers)
    }

    /// Batch validate sequences for multiple accounts
    pub fn batch_validate_sequences(&self, validations: &[(String, u64)]) -> Result<(), AccountError> {
        for (address, expected_sequence) in validations {
            self.validate_sequence(address, *expected_sequence)?;
        }
        Ok(())
    }

    /// Batch increment sequences for multiple accounts
    pub fn batch_increment_sequences(&mut self, addresses: &[String]) -> Result<Vec<u64>, AccountError> {
        let mut new_sequences = Vec::new();

        for address in addresses {
            let new_sequence = self.increment_sequence(address)?;
            new_sequences.push(new_sequence);
        }

        Ok(new_sequences)
    }

    /// Get total number of accounts
    pub fn get_account_count(&self) -> u64 {
        self.next_account_number - 1
    }

    /// List all accounts (for debugging/admin purposes)
    /// Note: This is a placeholder implementation since LookupMap doesn't support iteration
    pub fn list_accounts(&self, limit: Option<usize>) -> Vec<CosmosAccount> {
        let mut accounts = Vec::new();
        let max_items = limit.unwrap_or(self.account_addresses.len() as usize);
        
        for i in 0..std::cmp::min(max_items, self.account_addresses.len() as usize) {
            let address = self.account_addresses.get(i as u64).unwrap();
            if let Some(account) = self.accounts.get(&address) {
                accounts.push(account);
            }
        }
        
        accounts
    }

    /// Update configuration
    pub fn update_config(&mut self, config: AccountConfig) {
        self.config = config;
    }

    /// Get configuration
    pub fn get_config(&self) -> &AccountConfig {
        &self.config
    }

    /// Derive multiple addresses from public keys
    pub fn derive_addresses(&self, public_keys: &[CosmosPublicKey]) -> Result<Vec<String>, AccountError> {
        let mut addresses = Vec::new();

        for public_key in public_keys {
            let address = public_key.to_cosmos_address(&self.config.address_prefix)
                .map_err(|e| AccountError::AddressDerivationFailed(e.to_string()))?;
            addresses.push(address);
        }

        Ok(addresses)
    }

    /// Check if address format is valid
    pub fn is_valid_address(&self, address: &str) -> bool {
        // Simple validation - should start with the prefix
        address.starts_with(&format!("{}1", self.config.address_prefix)) ||
        // Also accept NEAR account IDs for compatibility
        address.ends_with(".near") || address.ends_with(".testnet")
    }

    /// Convert NEAR account ID to Cosmos-style address if needed
    pub fn normalize_address(&mut self, address: &str) -> Result<String, AccountError> {
        // If it's already a Cosmos address, return as-is
        if self.is_valid_address(address) && !address.contains(".") {
            return Ok(address.to_string());
        }

        // If it's a NEAR account ID, get or create the mapping
        if address.ends_with(".near") || address.ends_with(".testnet") {
            let near_account_id: AccountId = address.parse()
                .map_err(|_| AccountError::InvalidAddress(address.to_string()))?;

            if let Some(cosmos_address) = self.near_to_cosmos.get(&near_account_id) {
                Ok(cosmos_address)
            } else if self.config.auto_create_accounts {
                let account = self.create_account_from_near_id(near_account_id)?;
                Ok(account.address)
            } else {
                Err(AccountError::AccountNotFound(address.to_string()))
            }
        } else {
            Err(AccountError::InvalidAddress(address.to_string()))
        }
    }
}

/// Utility functions for account management
pub mod utils {
    use super::*;

    /// Extract addresses from public keys
    pub fn extract_addresses_from_pubkeys(
        public_keys: &[CosmosPublicKey],
        address_prefix: &str,
    ) -> Result<Vec<String>, AccountError> {
        let mut addresses = Vec::new();

        for public_key in public_keys {
            let address = public_key.to_cosmos_address(address_prefix)
                .map_err(|e| AccountError::AddressDerivationFailed(e.to_string()))?;
            addresses.push(address);
        }

        Ok(addresses)
    }

    /// Validate address format
    pub fn validate_address_format(address: &str, expected_prefix: &str) -> bool {
        address.starts_with(&format!("{}1", expected_prefix)) && address.len() > expected_prefix.len() + 10
    }

    /// Convert sequence validations to a map for easier lookup
    pub fn sequence_validations_to_map(validations: &[(String, u64)]) -> HashMap<String, u64> {
        validations.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::CosmosPublicKey;

    fn create_test_public_key() -> CosmosPublicKey {
        // Create a test secp256k1 public key (33 bytes compressed)
        let pub_key_bytes = vec![
            0x03, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20
        ];
        CosmosPublicKey::secp256k1(pub_key_bytes).unwrap()
    }

    #[test]
    fn test_cosmos_account_creation() {
        let account = CosmosAccount::new("test1address".to_string(), 42);
        
        assert_eq!(account.address, "test1address");
        assert_eq!(account.account_number, 42);
        assert_eq!(account.sequence, 0);
        assert!(!account.has_public_key());
    }

    #[test]
    fn test_account_sequence_operations() {
        let mut account = CosmosAccount::new("test1address".to_string(), 42);
        
        // Test sequence increment
        let new_seq = account.increment_sequence();
        assert_eq!(new_seq, 1);
        assert_eq!(account.sequence, 1);
        
        // Test sequence validation
        assert!(account.validate_sequence(1).is_ok());
        assert!(account.validate_sequence(0).is_err());
        assert!(account.validate_sequence(2).is_err());
    }

    #[test]
    fn test_account_manager_creation() {
        let config = AccountConfig::default();
        let manager = AccountManager::new(config.clone());
        
        assert_eq!(manager.get_account_count(), 0);
        assert_eq!(manager.get_config().address_prefix, "near");
    }

    #[test]
    fn test_account_creation_with_pubkey() {
        let mut manager = AccountManager::new(AccountConfig::default());
        let public_key = create_test_public_key();
        
        let account = manager.create_account(public_key).unwrap();
        
        assert_eq!(account.account_number, 1);
        assert_eq!(account.sequence, 0);
        assert!(account.has_public_key());
        assert_eq!(manager.get_account_count(), 1);
    }

    #[test]
    fn test_account_creation_from_near_id() {
        let mut manager = AccountManager::new(AccountConfig::default());
        let near_id: AccountId = "test.near".parse().unwrap();
        
        let account = manager.create_account_from_near_id(near_id.clone()).unwrap();
        
        assert_eq!(account.account_number, 1);
        assert_eq!(account.sequence, 0);
        assert_eq!(account.near_account_id, Some(near_id.clone()));
        
        // Should be able to retrieve by NEAR ID
        let retrieved = manager.get_account_by_near_id(&near_id).unwrap();
        assert_eq!(retrieved.account_number, 1);
    }

    #[test]
    fn test_duplicate_account_creation() {
        let mut manager = AccountManager::new(AccountConfig::default());
        let public_key = create_test_public_key();
        
        // First creation should succeed
        let _account1 = manager.create_account(public_key.clone()).unwrap();
        
        // Second creation should fail
        let result = manager.create_account(public_key);
        assert!(matches!(result, Err(AccountError::AccountExists(_))));
        
        // Account count should still be 1
        assert_eq!(manager.get_account_count(), 1);
    }

    #[test]
    fn test_sequence_operations() {
        let mut manager = AccountManager::new(AccountConfig::default());
        let public_key = create_test_public_key();
        let account = manager.create_account(public_key).unwrap();
        
        // Initial sequence validation
        assert!(manager.validate_sequence(&account.address, 0).is_ok());
        assert!(manager.validate_sequence(&account.address, 1).is_err());
        
        // Increment sequence
        let new_seq = manager.increment_sequence(&account.address).unwrap();
        assert_eq!(new_seq, 1);
        
        // Validate new sequence
        assert!(manager.validate_sequence(&account.address, 1).is_ok());
        assert!(manager.validate_sequence(&account.address, 0).is_err());
    }

    #[test]
    fn test_batch_operations() {
        let mut manager = AccountManager::new(AccountConfig::default());
        
        // Create multiple accounts
        let mut addresses = Vec::new();
        for i in 0..3 {
            let mut pub_key_bytes = vec![0x03; 33];
            pub_key_bytes[1] = i as u8; // Make each key unique
            let public_key = CosmosPublicKey::secp256k1(pub_key_bytes).unwrap();
            let account = manager.create_account(public_key).unwrap();
            addresses.push(account.address);
        }
        
        // Test batch sequence validation
        let validations: Vec<(String, u64)> = addresses.iter().map(|addr| (addr.clone(), 0)).collect();
        assert!(manager.batch_validate_sequences(&validations).is_ok());
        
        // Test batch sequence increment
        let new_sequences = manager.batch_increment_sequences(&addresses).unwrap();
        assert_eq!(new_sequences, vec![1, 1, 1]);
        
        // Validate sequences after increment
        let validations: Vec<(String, u64)> = addresses.iter().map(|addr| (addr.clone(), 1)).collect();
        assert!(manager.batch_validate_sequences(&validations).is_ok());
    }

    #[test]
    fn test_get_or_create_account() {
        let mut manager = AccountManager::new(AccountConfig::default());
        let public_key = create_test_public_key();
        
        // First call should create account
        let account1 = manager.get_or_create_account(public_key.clone()).unwrap();
        assert_eq!(account1.account_number, 1);
        
        // Second call should return existing account
        let account2 = manager.get_or_create_account(public_key).unwrap();
        assert_eq!(account2.account_number, 1);
        assert_eq!(account1.address, account2.address);
        
        // Only one account should exist
        assert_eq!(manager.get_account_count(), 1);
    }

    #[test]
    fn test_address_validation() {
        let manager = AccountManager::new(AccountConfig {
            address_prefix: "cosmos".to_string(),
            auto_create_accounts: true,
            max_sequence: 1000,
        });
        
        assert!(manager.is_valid_address("cosmos1abc123"));
        assert!(manager.is_valid_address("test.near"));
        assert!(manager.is_valid_address("account.testnet"));
        assert!(!manager.is_valid_address("invalid"));
        assert!(!manager.is_valid_address(""));
    }

    #[test]
    fn test_address_normalization() {
        let mut manager = AccountManager::new(AccountConfig::default());
        
        // Test NEAR account ID normalization
        let near_id = "test.near";
        let normalized = manager.normalize_address(near_id).unwrap();
        
        assert!(normalized.starts_with("near1"));
        assert_ne!(normalized, near_id);
        
        // Should be able to get the account now
        let account = manager.get_account(&normalized).unwrap();
        assert_eq!(account.near_account_id.as_ref().unwrap().as_str(), near_id);
    }

    #[test]
    fn test_account_listing() {
        let mut manager = AccountManager::new(AccountConfig::default());
        
        // Create several accounts
        for i in 0..5 {
            let mut pub_key_bytes = vec![0x03; 33];
            pub_key_bytes[1] = i as u8;
            let public_key = CosmosPublicKey::secp256k1(pub_key_bytes).unwrap();
            manager.create_account(public_key).unwrap();
        }
        
        // List all accounts (now works with Vector tracking)
        let all_accounts = manager.list_accounts(None);
        assert_eq!(all_accounts.len(), 5); // Now returns actual accounts
        
        // List limited accounts (now works with Vector tracking)
        let limited_accounts = manager.list_accounts(Some(3));
        assert_eq!(limited_accounts.len(), 3); // Now returns limited accounts
    }

    #[test]
    fn test_utility_functions() {
        let public_keys = vec![create_test_public_key()];
        let addresses = utils::extract_addresses_from_pubkeys(&public_keys, "test").unwrap();
        
        assert_eq!(addresses.len(), 1);
        assert!(addresses[0].starts_with("test1"));
        
        // Test address format validation
        assert!(utils::validate_address_format("cosmos1abcdefghijk", "cosmos"));
        assert!(!utils::validate_address_format("cosmos1", "cosmos"));
        assert!(!utils::validate_address_format("invalid", "cosmos"));
    }
}