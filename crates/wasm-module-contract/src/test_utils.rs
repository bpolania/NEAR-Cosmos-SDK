/// Test Utilities for CW20 Testing
/// 
/// Helper functions and mock data for testing CosmWasm contracts

use near_sdk::json_types::Base64VecU8;
use near_sdk::AccountId;
use serde_json::{json, Value};
use sha2::{Sha256, Digest};

/// Convert NEAR account to Cosmos address for testing
pub fn to_cosmos_address(account: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(account.as_bytes());
    let hash = hasher.finalize();
    // Take first 20 bytes and encode as hex (simplified for testing)
    format!("proxima1{}", hex::encode(&hash[..20]))
}

/// Generate a minimal valid WASM module for testing
/// This creates a WASM module with the basic structure but no actual functionality
pub fn generate_mock_wasm(size_kb: usize) -> Base64VecU8 {
    let mut wasm = vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
    ];
    
    // Add type section
    wasm.extend_from_slice(&[
        0x01, // Type section
        0x04, // Section size
        0x01, // Number of types
        0x60, // Function type
        0x00, // No parameters
        0x00, // No results
    ]);
    
    // Add function section
    wasm.extend_from_slice(&[
        0x03, // Function section
        0x02, // Section size
        0x01, // Number of functions
        0x00, // Function type index
    ]);
    
    // Add export section
    wasm.extend_from_slice(&[
        0x07, // Export section
        0x08, // Section size
        0x01, // Number of exports
        0x04, // Name length
        b'm', b'a', b'i', b'n', // "main"
        0x00, // Function export
        0x00, // Function index
    ]);
    
    // Add code section
    wasm.extend_from_slice(&[
        0x0a, // Code section
        0x04, // Section size
        0x01, // Number of functions
        0x02, // Function body size
        0x00, // No locals
        0x0b, // End opcode
    ]);
    
    // Pad to requested size
    let target_size = size_kb * 1024;
    if wasm.len() < target_size {
        // Add custom section with padding
        let padding_size = target_size - wasm.len() - 5; // Account for section header
        wasm.push(0x00); // Custom section
        
        // Encode section size (simplified for small sizes)
        let size_bytes = (padding_size as u32).to_le_bytes();
        wasm.push(size_bytes[0]);
        
        // Add padding data
        wasm.extend(vec![0x00; padding_size - 1]);
    }
    
    Base64VecU8::from(wasm)
}

/// Create a standard CW20 instantiation message with Cosmos addresses
pub fn create_cw20_init_msg(
    name: &str,
    symbol: &str,
    decimals: u8,
    initial_balances: Vec<(&str, &str)>,
    minter: Option<&str>,
    cap: Option<&str>,
) -> String {
    let balances: Vec<Value> = initial_balances
        .iter()
        .map(|(address, amount)| {
            // Convert NEAR addresses to Cosmos format if needed
            let cosmos_addr = if address.contains(".") {
                to_cosmos_address(address)
            } else if address.starts_with("proxima1") {
                address.to_string()
            } else {
                to_cosmos_address(address)
            };
            json!({
                "address": cosmos_addr,
                "amount": amount
            })
        })
        .collect();
    
    let mint = minter.map(|m| {
        let cosmos_minter = if m.contains(".") {
            to_cosmos_address(m)
        } else if m.starts_with("proxima1") {
            m.to_string()
        } else {
            to_cosmos_address(m)
        };
        json!({
            "minter": cosmos_minter,
            "cap": cap
        })
    });
    
    json!({
        "name": name,
        "symbol": symbol,
        "decimals": decimals,
        "initial_balances": balances,
        "mint": mint,
        "marketing": null
    }).to_string()
}

/// Create a transfer message
pub fn create_transfer_msg(recipient: &str, amount: &str) -> String {
    json!({
        "transfer": {
            "recipient": recipient,
            "amount": amount
        }
    }).to_string()
}

/// Create a mint message
pub fn create_mint_msg(recipient: &str, amount: &str) -> String {
    json!({
        "mint": {
            "recipient": recipient,
            "amount": amount
        }
    }).to_string()
}

/// Create a burn message
pub fn create_burn_msg(amount: &str) -> String {
    json!({
        "burn": {
            "amount": amount
        }
    }).to_string()
}

/// Create an increase allowance message
pub fn create_increase_allowance_msg(spender: &str, amount: &str, expires: Option<u64>) -> String {
    let expires_obj = expires.map(|h| {
        json!({
            "at_height": h
        })
    });
    
    json!({
        "increase_allowance": {
            "spender": spender,
            "amount": amount,
            "expires": expires_obj
        }
    }).to_string()
}

/// Create a decrease allowance message
pub fn create_decrease_allowance_msg(spender: &str, amount: &str, expires: Option<u64>) -> String {
    let expires_obj = expires.map(|h| {
        json!({
            "at_height": h
        })
    });
    
    json!({
        "decrease_allowance": {
            "spender": spender,
            "amount": amount,
            "expires": expires_obj
        }
    }).to_string()
}

/// Create a transfer from message
pub fn create_transfer_from_msg(owner: &str, recipient: &str, amount: &str) -> String {
    json!({
        "transfer_from": {
            "owner": owner,
            "recipient": recipient,
            "amount": amount
        }
    }).to_string()
}

/// Create a burn from message
pub fn create_burn_from_msg(owner: &str, amount: &str) -> String {
    json!({
        "burn_from": {
            "owner": owner,
            "amount": amount
        }
    }).to_string()
}

/// Create a send message (transfer with callback)
pub fn create_send_msg(contract: &str, amount: &str, msg: Vec<u8>) -> String {
    json!({
        "send": {
            "contract": contract,
            "amount": amount,
            "msg": Base64VecU8::from(msg)
        }
    }).to_string()
}

/// Query messages
pub mod queries {
    use serde_json::json;
    
    pub fn token_info() -> String {
        json!({"token_info": {}}).to_string()
    }
    
    pub fn balance(address: &str) -> String {
        json!({
            "balance": {
                "address": address
            }
        }).to_string()
    }
    
    pub fn minter() -> String {
        json!({"minter": {}}).to_string()
    }
    
    pub fn allowance(owner: &str, spender: &str) -> String {
        json!({
            "allowance": {
                "owner": owner,
                "spender": spender
            }
        }).to_string()
    }
    
    pub fn all_allowances(owner: &str, start_after: Option<&str>, limit: Option<u32>) -> String {
        json!({
            "all_allowances": {
                "owner": owner,
                "start_after": start_after,
                "limit": limit
            }
        }).to_string()
    }
    
    pub fn all_accounts(start_after: Option<&str>, limit: Option<u32>) -> String {
        json!({
            "all_accounts": {
                "start_after": start_after,
                "limit": limit
            }
        }).to_string()
    }
    
    pub fn marketing_info() -> String {
        json!({"marketing_info": {}}).to_string()
    }
    
    pub fn download_logo() -> String {
        json!({"download_logo": {}}).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_mock_wasm() {
        let wasm = generate_mock_wasm(1);
        let bytes: Vec<u8> = wasm.into();
        
        // Check WASM magic number
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
        
        // Check size is approximately 1KB
        assert!(bytes.len() >= 1024);
        assert!(bytes.len() < 1100); // Allow some overhead
    }
    
    #[test]
    fn test_create_cw20_init_msg() {
        let msg = create_cw20_init_msg(
            "Test Token",
            "TEST",
            6,
            vec![("alice", "1000000"), ("bob", "500000")],
            Some("minter"),
            Some("10000000"),
        );
        
        assert!(msg.contains("Test Token"));
        assert!(msg.contains("TEST"));
        assert!(msg.contains("alice"));
        assert!(msg.contains("1000000"));
        assert!(msg.contains("minter"));
    }
    
    #[test]
    fn test_create_transfer_msg() {
        let msg = create_transfer_msg("recipient", "100000");
        
        assert!(msg.contains("transfer"));
        assert!(msg.contains("recipient"));
        assert!(msg.contains("100000"));
    }
    
    #[test]
    fn test_query_messages() {
        let token_info = queries::token_info();
        assert!(token_info.contains("token_info"));
        
        let balance = queries::balance("test_address");
        assert!(balance.contains("balance"));
        assert!(balance.contains("test_address"));
        
        let allowance = queries::allowance("owner", "spender");
        assert!(allowance.contains("allowance"));
        assert!(allowance.contains("owner"));
        assert!(allowance.contains("spender"));
    }
}