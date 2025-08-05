use crate::modules::cosmwasm::types::{Api, Addr, StdResult, StdError};
use near_sdk::env;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

/// CosmWasm API implementation for NEAR
#[derive(BorshDeserialize, BorshSerialize)]
pub struct CosmWasmApi;

impl CosmWasmApi {
    pub fn new() -> Self {
        CosmWasmApi
    }
}

impl Api for CosmWasmApi {
    /// Validate an address (supports both NEAR and Cosmos formats)
    fn addr_validate(&self, human: &str) -> StdResult<Addr> {
        // Check if it's a valid NEAR address
        if human.ends_with(".near") || human.ends_with(".testnet") {
            // Basic NEAR address validation
            if human.len() >= 2 && human.len() <= 64 {
                return Ok(Addr::unchecked(human));
            }
        }
        
        // Check if it's a valid implicit NEAR account (64 hex chars)
        if human.len() == 64 && human.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(Addr::unchecked(human));
        }
        
        // Check if it's a Cosmos address
        if human.starts_with("cosmos1") && human.len() == 45 {
            // Basic Cosmos address validation
            // In production, would validate bech32 encoding
            return Ok(Addr::unchecked(human));
        }
        
        // Check if it's a Proxima-specific address format
        if human.starts_with("proxima1") && human.len() == 46 {
            return Ok(Addr::unchecked(human));
        }
        
        // Allow simple test addresses (alphanumeric, 2-32 chars for testing)
        if cfg!(test) && human.len() >= 2 && human.len() <= 32 && human.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Ok(Addr::unchecked(human));
        }
        
        Err(StdError::generic_err(format!(
            "Invalid address format: {}. Expected NEAR account, Cosmos address, or Proxima address",
            human
        )))
    }
    
    /// Convert human address to canonical form
    fn addr_canonicalize(&self, human: &str) -> StdResult<Vec<u8>> {
        // For NEAR addresses, we'll use the UTF-8 bytes as canonical form
        // This maintains compatibility while allowing cross-ecosystem addresses
        let validated = self.addr_validate(human)?;
        Ok(validated.as_str().as_bytes().to_vec())
    }
    
    /// Convert canonical address back to human form
    fn addr_humanize(&self, canonical: &[u8]) -> StdResult<Addr> {
        // Convert bytes back to string
        let human = String::from_utf8(canonical.to_vec())
            .map_err(|e| StdError::invalid_utf8(e.to_string()))?;
        
        // Validate the address
        self.addr_validate(&human)
    }
    
    /// Verify secp256k1 signature
    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> StdResult<bool> {
        // Validate input lengths
        if message_hash.len() != 32 {
            return Err(StdError::generic_err("Invalid message hash length: expected 32 bytes"));
        }
        
        if signature.len() != 64 {
            return Err(StdError::generic_err("Invalid signature length: expected 64 bytes"));
        }
        
        if public_key.len() != 33 && public_key.len() != 65 {
            return Err(StdError::generic_err(
                "Invalid public key length: expected 33 (compressed) or 65 (uncompressed) bytes"
            ));
        }
        
        // For now, we'll implement a simplified version
        // In production, this would use the k256 crate for actual secp256k1 verification
        
        // TODO: Implement actual secp256k1 verification using k256 crate
        // This is a placeholder that always returns true for testing
        
        // Log warning about using placeholder implementation
        env::log_str("WARNING: Using placeholder secp256k1_verify implementation");
        
        Ok(true)
    }
    
    /// Recover public key from secp256k1 signature
    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_id: u8,
    ) -> StdResult<Vec<u8>> {
        // Validate inputs
        if message_hash.len() != 32 {
            return Err(StdError::generic_err("Invalid message hash length: expected 32 bytes"));
        }
        
        if signature.len() != 64 {
            return Err(StdError::generic_err("Invalid signature length: expected 64 bytes"));
        }
        
        if recovery_id > 3 {
            return Err(StdError::generic_err("Invalid recovery ID: must be 0-3"));
        }
        
        // TODO: Implement actual secp256k1 recovery using k256 crate
        // This is a placeholder that returns a dummy public key for testing
        
        // Log warning about using placeholder implementation
        env::log_str("WARNING: Using placeholder secp256k1_recover_pubkey implementation");
        
        // Return a dummy compressed public key (33 bytes)
        Ok(vec![0x02; 33])
    }
    
    /// Verify Ed25519 signature using NEAR's native implementation
    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> StdResult<bool> {
        // Validate input lengths
        if signature.len() != 64 {
            return Err(StdError::generic_err("Invalid signature length: expected 64 bytes"));
        }
        
        if public_key.len() != 32 {
            return Err(StdError::generic_err("Invalid public key length: expected 32 bytes"));
        }
        
        // Convert to fixed-size arrays for NEAR's API
        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(signature);
        
        let mut pubkey_array = [0u8; 32];
        pubkey_array.copy_from_slice(public_key);
        
        // Use NEAR's native Ed25519 verification
        Ok(env::ed25519_verify(&sig_array, message, &pubkey_array))
    }
}

/// Additional cryptographic utilities
impl CosmWasmApi {
    /// Hash data using SHA256 (using NEAR's native implementation)
    pub fn sha256(&self, data: &[u8]) -> Vec<u8> {
        env::sha256(data).to_vec()
    }
    
    /// Hash data using Keccak256 (using NEAR's native implementation)
    pub fn keccak256(&self, data: &[u8]) -> Vec<u8> {
        env::keccak256(data).to_vec()
    }
    
    /// Generate a Cosmos-style address from a public key
    pub fn pubkey_to_cosmos_addr(&self, public_key: &[u8], prefix: &str) -> StdResult<String> {
        // Step 1: SHA256 hash of the public key
        let sha_hash = self.sha256(public_key);
        
        // Step 2: RIPEMD160 of the SHA256 hash
        // Since NEAR doesn't have native RIPEMD160, we'll use a simplified version
        // In production, this would use the ripemd crate
        let addr_bytes = &sha_hash[0..20]; // Take first 20 bytes as simplified RIPEMD160
        
        // Step 3: Bech32 encoding with the given prefix
        // For now, return a simplified format
        // In production, this would use the bech32 crate
        Ok(format!("{}1{}", prefix, hex::encode(addr_bytes)))
    }
    
    /// Verify a batch of Ed25519 signatures
    pub fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> StdResult<bool> {
        // Validate input lengths match
        if messages.len() != signatures.len() || messages.len() != public_keys.len() {
            return Err(StdError::generic_err("Batch verification: mismatched array lengths"));
        }
        
        // Verify each signature individually
        // In production, could optimize with actual batch verification
        for i in 0..messages.len() {
            // Validate and convert each signature and public key
            if signatures[i].len() != 64 {
                return Err(StdError::generic_err("Invalid signature length in batch"));
            }
            if public_keys[i].len() != 32 {
                return Err(StdError::generic_err("Invalid public key length in batch"));
            }
            
            let mut sig_array = [0u8; 64];
            sig_array.copy_from_slice(signatures[i]);
            
            let mut pubkey_array = [0u8; 32];
            pubkey_array.copy_from_slice(public_keys[i]);
            
            if !env::ed25519_verify(&sig_array, messages[i], &pubkey_array) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_addr_validation() {
        let api = CosmWasmApi::new();
        
        // Test NEAR addresses
        assert!(api.addr_validate("alice.near").is_ok());
        assert!(api.addr_validate("alice.testnet").is_ok());
        assert!(api.addr_validate("a".repeat(64).as_str()).is_ok()); // Implicit account
        
        // Test Cosmos addresses
        assert!(api.addr_validate("cosmos1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzv7xu").is_ok());
        
        // Test Proxima addresses
        assert!(api.addr_validate("proxima1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzv7xu2").is_ok());
        
        // Test invalid addresses
        assert!(api.addr_validate("").is_err());
        assert!(api.addr_validate("invalid").is_err());
        assert!(api.addr_validate("toolong".repeat(20).as_str()).is_err());
    }
    
    #[test]
    fn test_addr_canonicalize_humanize() {
        let api = CosmWasmApi::new();
        
        let addresses = vec![
            "alice.near",
            "bob.testnet",
            "cosmos1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzv7xu",
        ];
        
        for addr in addresses {
            let canonical = api.addr_canonicalize(addr).unwrap();
            let human = api.addr_humanize(&canonical).unwrap();
            assert_eq!(human.as_str(), addr);
        }
    }
    
    #[test]
    fn test_sha256() {
        let api = CosmWasmApi::new();
        
        let data = b"hello world";
        let hash = api.sha256(data);
        
        // SHA256 of "hello world"
        let expected = hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9").unwrap();
        assert_eq!(hash, expected);
    }
}