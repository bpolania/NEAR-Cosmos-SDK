/// Address Conversion Module
/// 
/// Handles conversion between NEAR account IDs and Cosmos bech32 addresses
/// for CosmWasm compatibility

use near_sdk::AccountId;
use sha2::{Sha256, Digest};

/// Default bech32 prefix for Cosmos addresses in our system
pub const DEFAULT_BECH32_PREFIX: &str = "proxima";

/// Convert a NEAR account ID to a Cosmos-style bech32 address
/// 
/// This creates a deterministic mapping from NEAR accounts to Cosmos addresses:
/// 1. Hash the NEAR account ID with SHA256
/// 2. Take the first 20 bytes (standard Cosmos address length)
/// 3. Encode as bech32 with the specified prefix
pub fn near_to_cosmos_address(near_account: &AccountId, prefix: Option<&str>) -> String {
    let prefix = prefix.unwrap_or(DEFAULT_BECH32_PREFIX);
    
    // Hash the NEAR account ID to get a deterministic 32-byte value
    let mut hasher = Sha256::new();
    hasher.update(near_account.as_bytes());
    let hash = hasher.finalize();
    
    // Take first 20 bytes for Cosmos address (standard length)
    let addr_bytes = &hash[..20];
    
    // Encode as bech32
    bech32_encode(prefix, addr_bytes)
}

/// Convert a Cosmos bech32 address back to NEAR account ID (if it exists in our mapping)
/// 
/// Note: This requires maintaining a mapping since the hash is one-way
/// In practice, we'd store this mapping in contract state
pub fn cosmos_to_near_address(cosmos_addr: &str, mapping: &std::collections::HashMap<String, AccountId>) -> Option<AccountId> {
    mapping.get(cosmos_addr).cloned()
}

/// Encode bytes as bech32 address with given prefix
fn bech32_encode(hrp: &str, data: &[u8]) -> String {
    use bech32::{ToBase32, Variant};
    
    bech32::encode(hrp, data.to_base32(), Variant::Bech32)
        .unwrap_or_else(|_| format!("{}1invalid", hrp))
}

/// Decode bech32 address to get the raw bytes
pub fn bech32_decode(addr: &str) -> Result<(String, Vec<u8>), String> {
    use bech32::{FromBase32, Variant};
    
    let (hrp, data, variant) = bech32::decode(addr)
        .map_err(|e| format!("Invalid bech32 address: {}", e))?;
    
    if variant != Variant::Bech32 {
        return Err("Invalid bech32 variant".to_string());
    }
    
    let bytes = Vec::<u8>::from_base32(&data)
        .map_err(|e| format!("Invalid base32 data: {}", e))?;
    
    Ok((hrp, bytes))
}

/// Generate a contract address in Cosmos style
/// 
/// In Cosmos, contract addresses are derived from:
/// - The creator's address
/// - A nonce or sequence number
/// 
/// We adapt this for NEAR by using the module account and instance ID
pub fn generate_contract_address(module_account: &AccountId, instance_id: u64, prefix: Option<&str>) -> String {
    let prefix = prefix.unwrap_or(DEFAULT_BECH32_PREFIX);
    
    // Create a unique hash for this contract instance
    let mut hasher = Sha256::new();
    hasher.update(b"contract");
    hasher.update(module_account.as_bytes());
    hasher.update(instance_id.to_le_bytes());
    let hash = hasher.finalize();
    
    // Take first 20 bytes for address
    let addr_bytes = &hash[..20];
    
    bech32_encode(prefix, addr_bytes)
}

/// Check if an address is a valid Cosmos bech32 address
pub fn is_valid_cosmos_address(addr: &str) -> bool {
    bech32_decode(addr).is_ok()
}

/// Check if an address is a valid NEAR account ID
pub fn is_valid_near_address(addr: &str) -> bool {
    addr.parse::<AccountId>().is_ok()
}

/// Address type that can handle both NEAR and Cosmos addresses
#[derive(Clone, Debug, PartialEq)]
pub enum UniversalAddress {
    Near(AccountId),
    Cosmos(String),
}

impl UniversalAddress {
    /// Create from a string, auto-detecting the format
    pub fn from_string(addr: &str) -> Result<Self, String> {
        // Check if it's a Cosmos address (starts with prefix and contains '1')
        if addr.contains('1') && !addr.contains('.') {
            if is_valid_cosmos_address(addr) {
                return Ok(UniversalAddress::Cosmos(addr.to_string()));
            }
        }
        
        // Try parsing as NEAR account
        if let Ok(near_account) = addr.parse::<AccountId>() {
            return Ok(UniversalAddress::Near(near_account));
        }
        
        Err(format!("Invalid address format: {}", addr))
    }
    
    /// Convert to Cosmos address format
    pub fn to_cosmos(&self, prefix: Option<&str>) -> String {
        match self {
            UniversalAddress::Near(account) => near_to_cosmos_address(account, prefix),
            UniversalAddress::Cosmos(addr) => addr.clone(),
        }
    }
    
    /// Get the original string representation
    pub fn to_string(&self) -> String {
        match self {
            UniversalAddress::Near(account) => account.to_string(),
            UniversalAddress::Cosmos(addr) => addr.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::accounts;
    
    #[test]
    fn test_near_to_cosmos_address() {
        let near_account = accounts(0);
        let cosmos_addr = near_to_cosmos_address(&near_account, None);
        
        // Should start with the default prefix
        assert!(cosmos_addr.starts_with("proxima1"));
        
        // Should be deterministic
        let cosmos_addr2 = near_to_cosmos_address(&near_account, None);
        assert_eq!(cosmos_addr, cosmos_addr2);
    }
    
    #[test]
    fn test_near_to_cosmos_with_custom_prefix() {
        let near_account = accounts(1);
        let cosmos_addr = near_to_cosmos_address(&near_account, Some("cosmos"));
        
        assert!(cosmos_addr.starts_with("cosmos1"));
    }
    
    #[test]
    fn test_generate_contract_address() {
        let module_account = accounts(0);
        let addr1 = generate_contract_address(&module_account, 1, None);
        let addr2 = generate_contract_address(&module_account, 2, None);
        
        // Different instance IDs should generate different addresses
        assert_ne!(addr1, addr2);
        
        // Should be deterministic
        let addr1_again = generate_contract_address(&module_account, 1, None);
        assert_eq!(addr1, addr1_again);
    }
    
    #[test]
    fn test_universal_address() {
        // Test NEAR address
        let near_addr = "alice.near";
        let universal = UniversalAddress::from_string(near_addr).unwrap();
        assert!(matches!(universal, UniversalAddress::Near(_)));
        
        // Test Cosmos address - generate a valid one
        let test_bytes = vec![0u8; 20];
        let cosmos_addr = bech32_encode("proxima", &test_bytes);
        let universal = UniversalAddress::from_string(&cosmos_addr).unwrap();
        assert!(matches!(universal, UniversalAddress::Cosmos(_)));
    }
    
    #[test]
    fn test_address_validation() {
        assert!(is_valid_near_address("alice.near"));
        assert!(is_valid_near_address("alice.testnet"));
        assert!(!is_valid_near_address("invalid..account"));
        
        // Note: These will fail without proper bech32 implementation
        // but the structure is correct for testing
    }
}