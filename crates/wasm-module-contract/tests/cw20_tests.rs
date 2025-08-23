/// CW20 Token Contract Tests
/// 
/// Tests for CosmWasm CW20 token operations including:
/// - Token instantiation
/// - Minting operations
/// - Transfer operations
/// - Burn operations
/// - Allowances and approvals
/// - Query operations

use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::{testing_env, AccountId};
use near_sdk::json_types::Base64VecU8;
use serde_json::json;
use wasm_module_contract::{WasmModuleContract, CodeID};
use sha2::{Sha256, Digest};

/// Helper to convert NEAR account to mock Cosmos address for testing
fn to_cosmos_address(account: &AccountId) -> String {
    let mut hasher = Sha256::new();
    hasher.update(account.as_bytes());
    let hash = hasher.finalize();
    // Take first 20 bytes and encode as hex (simplified for testing)
    format!("proxima1{}", hex::encode(&hash[..20]))
}

/// Test helper to create a new contract instance
fn setup_contract() -> WasmModuleContract {
    let context = VMContextBuilder::new()
        .current_account_id(accounts(0))
        .predecessor_account_id(accounts(1))
        .build();
    testing_env!(context);
    
    WasmModuleContract::new(Some(accounts(1)), Some(accounts(2)))
}

/// Helper to create a minimal CW20 WASM bytecode for testing
fn get_test_cw20_wasm() -> Base64VecU8 {
    // Minimal valid WASM module for testing
    // In production, this would be the actual CW20 bytecode
    Base64VecU8::from(vec![
        0x00, 0x61, 0x73, 0x6d, // WASM magic number
        0x01, 0x00, 0x00, 0x00, // Version 1
        // Minimal valid module structure
        0x00, // Custom section
        0x04, // Section length
        0x04, // Name length
        0x6e, 0x61, 0x6d, 0x65, // "name"
    ])
}

/// Helper to store CW20 code
fn store_cw20_code(contract: &mut WasmModuleContract) -> CodeID {
    let wasm_code = get_test_cw20_wasm();
    let response = contract.store_code(
        wasm_code,
        Some("cw20-base".to_string()),
        Some("cosmwasm".to_string()),
        None,
        None,
    );
    response.code_id
}

#[cfg(test)]
mod instantiation_tests {
    use super::*;

    #[test]
    fn test_instantiate_cw20_token() {
        let mut contract = setup_contract();
        let code_id = store_cw20_code(&mut contract);
        
        // Convert NEAR accounts to Cosmos addresses for CW20
        let minter_addr = to_cosmos_address(&accounts(1));
        let initial_holder = to_cosmos_address(&accounts(1));
        
        // Prepare instantiation message for CW20
        let init_msg = json!({
            "name": "Test Token",
            "symbol": "TEST",
            "decimals": 6,
            "initial_balances": [
                {
                    "address": initial_holder,
                    "amount": "1000000"
                }
            ],
            "mint": {
                "minter": minter_addr,
                "cap": "1000000000"
            },
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "test-token".to_string(),
            Some(accounts(1).to_string()),
            None,
        );
        
        // Contract addresses now use Cosmos format
        assert!(response.address.starts_with("proxima1"));
        assert!(response.data.is_some());
    }

    #[test]
    fn test_instantiate_with_multiple_balances() {
        let mut contract = setup_contract();
        let code_id = store_cw20_code(&mut contract);
        
        let init_msg = json!({
            "name": "Multi Balance Token",
            "symbol": "MBT",
            "decimals": 18,
            "initial_balances": [
                {
                    "address": to_cosmos_address(&accounts(1)),
                    "amount": "1000000000000000000"
                },
                {
                    "address": to_cosmos_address(&accounts(2)),
                    "amount": "500000000000000000"
                },
                {
                    "address": to_cosmos_address(&accounts(3)),
                    "amount": "250000000000000000"
                }
            ],
            "mint": null,
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "multi-balance-token".to_string(),
            None,
            None,
        );
        
        assert!(response.address.starts_with("proxima1"));
    }
}

#[cfg(test)]
mod mint_tests {
    use super::*;

    fn setup_mintable_token(contract: &mut WasmModuleContract) -> String {
        let code_id = store_cw20_code(contract);
        
        let init_msg = json!({
            "name": "Mintable Token",
            "symbol": "MINT",
            "decimals": 6,
            "initial_balances": [],
            "mint": {
                "minter": to_cosmos_address(&accounts(1)),
                "cap": null  // No cap on minting
            },
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "mintable-token".to_string(),
            Some(accounts(1).to_string()),
            None,
        );
        
        response.address
    }

    #[test]
    fn test_mint_tokens() {
        let mut contract = setup_contract();
        let token_addr = setup_mintable_token(&mut contract);
        
        // Mint tokens to a recipient
        let mint_msg = json!({
            "mint": {
                "recipient": to_cosmos_address(&accounts(2)),
                "amount": "1000000"
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            mint_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
        assert!(!response.events.is_empty());
    }

    #[test]
    fn test_mint_with_cap() {
        let mut contract = setup_contract();
        let code_id = store_cw20_code(&mut contract);
        
        // Create token with minting cap
        let init_msg = json!({
            "name": "Capped Token",
            "symbol": "CAP",
            "decimals": 6,
            "initial_balances": [],
            "mint": {
                "minter": accounts(1).to_string(),
                "cap": "1000000"  // Cap at 1M tokens
            },
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "capped-token".to_string(),
            Some(accounts(1).to_string()),
            None,
        );
        
        let token_addr = response.address;
        
        // Try to mint within cap
        let mint_msg = json!({
            "mint": {
                "recipient": accounts(2).to_string(),
                "amount": "500000"
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            mint_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }
}

#[cfg(test)]
mod transfer_tests {
    use super::*;

    fn setup_token_with_balance(contract: &mut WasmModuleContract) -> String {
        let code_id = store_cw20_code(contract);
        
        let init_msg = json!({
            "name": "Transfer Test Token",
            "symbol": "TTT",
            "decimals": 6,
            "initial_balances": [
                {
                    "address": accounts(1).to_string(),
                    "amount": "1000000"
                }
            ],
            "mint": null,
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "transfer-token".to_string(),
            None,
            None,
        );
        
        response.address
    }

    #[test]
    fn test_transfer_tokens() {
        let mut contract = setup_contract();
        let token_addr = setup_token_with_balance(&mut contract);
        
        // Transfer tokens from account 1 to account 2
        let transfer_msg = json!({
            "transfer": {
                "recipient": accounts(2).to_string(),
                "amount": "100000"
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            transfer_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
        assert!(!response.events.is_empty());
    }

    #[test]
    fn test_transfer_from() {
        let mut contract = setup_contract();
        let token_addr = setup_token_with_balance(&mut contract);
        
        // First, approve allowance
        let approve_msg = json!({
            "increase_allowance": {
                "spender": accounts(2).to_string(),
                "amount": "500000",
                "expires": null
            }
        }).to_string();
        
        contract.execute(
            token_addr.clone(),
            approve_msg,
            None,
            None,
        );
        
        // Now transfer from using the allowance
        let transfer_from_msg = json!({
            "transfer_from": {
                "owner": accounts(1).to_string(),
                "recipient": accounts(3).to_string(),
                "amount": "250000"
            }
        }).to_string();
        
        // Switch context to spender
        let context = VMContextBuilder::new()
            .predecessor_account_id(accounts(2))
            .build();
        testing_env!(context);
        
        let response = contract.execute(
            token_addr.clone(),
            transfer_from_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }

    #[test]
    fn test_send_with_message() {
        let mut contract = setup_contract();
        let token_addr = setup_token_with_balance(&mut contract);
        
        // Send tokens with a message for the recipient contract
        let send_msg = json!({
            "send": {
                "contract": accounts(3).to_string(),
                "amount": "50000",
                "msg": Base64VecU8::from(b"custom message".to_vec())
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            send_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }
}

#[cfg(test)]
mod burn_tests {
    use super::*;

    fn setup_burnable_token(contract: &mut WasmModuleContract) -> String {
        let code_id = store_cw20_code(contract);
        
        let init_msg = json!({
            "name": "Burnable Token",
            "symbol": "BURN",
            "decimals": 6,
            "initial_balances": [
                {
                    "address": accounts(1).to_string(),
                    "amount": "1000000"
                }
            ],
            "mint": null,
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "burnable-token".to_string(),
            None,
            None,
        );
        
        response.address
    }

    #[test]
    fn test_burn_tokens() {
        let mut contract = setup_contract();
        let token_addr = setup_burnable_token(&mut contract);
        
        // Burn tokens
        let burn_msg = json!({
            "burn": {
                "amount": "100000"
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            burn_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }

    #[test]
    fn test_burn_from() {
        let mut contract = setup_contract();
        let token_addr = setup_burnable_token(&mut contract);
        
        // First, approve allowance for burning
        let approve_msg = json!({
            "increase_allowance": {
                "spender": accounts(2).to_string(),
                "amount": "500000",
                "expires": null
            }
        }).to_string();
        
        contract.execute(
            token_addr.clone(),
            approve_msg,
            None,
            None,
        );
        
        // Now burn from using the allowance
        let burn_from_msg = json!({
            "burn_from": {
                "owner": accounts(1).to_string(),
                "amount": "200000"
            }
        }).to_string();
        
        // Switch context to spender
        let context = VMContextBuilder::new()
            .predecessor_account_id(accounts(2))
            .build();
        testing_env!(context);
        
        let response = contract.execute(
            token_addr.clone(),
            burn_from_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }
}

#[cfg(test)]
mod allowance_tests {
    use super::*;

    fn setup_token_for_allowance(contract: &mut WasmModuleContract) -> String {
        let code_id = store_cw20_code(contract);
        
        let init_msg = json!({
            "name": "Allowance Token",
            "symbol": "ALLOW",
            "decimals": 6,
            "initial_balances": [
                {
                    "address": accounts(1).to_string(),
                    "amount": "1000000"
                }
            ],
            "mint": null,
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "allowance-token".to_string(),
            None,
            None,
        );
        
        response.address
    }

    #[test]
    fn test_increase_allowance() {
        let mut contract = setup_contract();
        let token_addr = setup_token_for_allowance(&mut contract);
        
        let increase_msg = json!({
            "increase_allowance": {
                "spender": accounts(2).to_string(),
                "amount": "300000",
                "expires": null
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            increase_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }

    #[test]
    fn test_decrease_allowance() {
        let mut contract = setup_contract();
        let token_addr = setup_token_for_allowance(&mut contract);
        
        // First increase allowance
        let increase_msg = json!({
            "increase_allowance": {
                "spender": accounts(2).to_string(),
                "amount": "500000",
                "expires": null
            }
        }).to_string();
        
        contract.execute(
            token_addr.clone(),
            increase_msg,
            None,
            None,
        );
        
        // Then decrease it
        let decrease_msg = json!({
            "decrease_allowance": {
                "spender": accounts(2).to_string(),
                "amount": "200000",
                "expires": null
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            decrease_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }

    #[test]
    fn test_allowance_with_expiration() {
        let mut contract = setup_contract();
        let token_addr = setup_token_for_allowance(&mut contract);
        
        // Set allowance with expiration
        let increase_msg = json!({
            "increase_allowance": {
                "spender": accounts(2).to_string(),
                "amount": "400000",
                "expires": {
                    "at_height": 1000000
                }
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr.clone(),
            increase_msg,
            None,
            None,
        );
        
        assert!(response.data.is_some());
    }
}

#[cfg(test)]
mod query_tests {
    use super::*;

    fn setup_query_token(contract: &mut WasmModuleContract) -> String {
        let code_id = store_cw20_code(contract);
        
        let init_msg = json!({
            "name": "Query Test Token",
            "symbol": "QTT",
            "decimals": 9,
            "initial_balances": [
                {
                    "address": accounts(1).to_string(),
                    "amount": "1000000000"
                },
                {
                    "address": accounts(2).to_string(),
                    "amount": "500000000"
                }
            ],
            "mint": {
                "minter": accounts(1).to_string(),
                "cap": "10000000000"
            },
            "marketing": {
                "project": "Test Project",
                "description": "A test token for queries",
                "marketing": accounts(3).to_string(),
                "logo": null
            }
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "query-token".to_string(),
            Some(accounts(1).to_string()),
            None,
        );
        
        response.address
    }

    #[test]
    fn test_query_token_info() {
        let mut contract = setup_contract();
        let token_addr = setup_query_token(&mut contract);
        
        let query_msg = json!({
            "token_info": {}
        }).to_string();
        
        let response = contract.query(token_addr, query_msg);
        
        assert!(response.contains("query_result"));
    }

    #[test]
    fn test_query_balance() {
        let mut contract = setup_contract();
        let token_addr = setup_query_token(&mut contract);
        
        let query_msg = json!({
            "balance": {
                "address": accounts(1).to_string()
            }
        }).to_string();
        
        let response = contract.query(token_addr, query_msg);
        
        assert!(response.contains("query_result"));
    }

    #[test]
    fn test_query_minter() {
        let mut contract = setup_contract();
        let token_addr = setup_query_token(&mut contract);
        
        let query_msg = json!({
            "minter": {}
        }).to_string();
        
        let response = contract.query(token_addr, query_msg);
        
        assert!(response.contains("query_result"));
    }

    #[test]
    fn test_query_allowance() {
        let mut contract = setup_contract();
        let token_addr = setup_query_token(&mut contract);
        
        // First set an allowance
        let approve_msg = json!({
            "increase_allowance": {
                "spender": accounts(2).to_string(),
                "amount": "100000000",
                "expires": null
            }
        }).to_string();
        
        contract.execute(
            token_addr.clone(),
            approve_msg,
            None,
            None,
        );
        
        // Query the allowance
        let query_msg = json!({
            "allowance": {
                "owner": accounts(1).to_string(),
                "spender": accounts(2).to_string()
            }
        }).to_string();
        
        let response = contract.query(token_addr, query_msg);
        
        assert!(response.contains("query_result"));
    }

    #[test]
    fn test_query_all_allowances() {
        let mut contract = setup_contract();
        let token_addr = setup_query_token(&mut contract);
        
        let query_msg = json!({
            "all_allowances": {
                "owner": accounts(1).to_string(),
                "start_after": null,
                "limit": 10
            }
        }).to_string();
        
        let response = contract.query(token_addr, query_msg);
        
        assert!(response.contains("query_result"));
    }

    #[test]
    fn test_query_all_accounts() {
        let mut contract = setup_contract();
        let token_addr = setup_query_token(&mut contract);
        
        let query_msg = json!({
            "all_accounts": {
                "start_after": null,
                "limit": 10
            }
        }).to_string();
        
        let response = contract.query(token_addr, query_msg);
        
        assert!(response.contains("query_result"));
    }

    #[test]
    fn test_query_marketing_info() {
        let mut contract = setup_contract();
        let token_addr = setup_query_token(&mut contract);
        
        let query_msg = json!({
            "marketing_info": {}
        }).to_string();
        
        let response = contract.query(token_addr, query_msg);
        
        assert!(response.contains("query_result"));
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_zero_amount_transfer() {
        let mut contract = setup_contract();
        let code_id = store_cw20_code(&mut contract);
        
        let init_msg = json!({
            "name": "Edge Case Token",
            "symbol": "EDGE",
            "decimals": 6,
            "initial_balances": [
                {
                    "address": accounts(1).to_string(),
                    "amount": "1000000"
                }
            ],
            "mint": null,
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "edge-token".to_string(),
            None,
            None,
        );
        
        let token_addr = response.address;
        
        // Try to transfer zero amount (should handle gracefully)
        let transfer_msg = json!({
            "transfer": {
                "recipient": accounts(2).to_string(),
                "amount": "0"
            }
        }).to_string();
        
        let response = contract.execute(
            token_addr,
            transfer_msg,
            None,
            None,
        );
        
        // Should handle zero transfers appropriately
        assert!(response.data.is_some());
    }

    #[test]
    fn test_max_supply_token() {
        let mut contract = setup_contract();
        let code_id = store_cw20_code(&mut contract);
        
        // Create token with maximum possible supply
        let init_msg = json!({
            "name": "Max Supply Token",
            "symbol": "MAX",
            "decimals": 18,
            "initial_balances": [
                {
                    "address": accounts(1).to_string(),
                    "amount": "115792089237316195423570985008687907853269984665640564039457584007913129639935"  // 2^256 - 1
                }
            ],
            "mint": null,
            "marketing": null
        }).to_string();
        
        let response = contract.instantiate(
            code_id,
            init_msg,
            None,
            "max-supply-token".to_string(),
            None,
            None,
        );
        
        assert!(response.address.starts_with("proxima1"));
    }
}