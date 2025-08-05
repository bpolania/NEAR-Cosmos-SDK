use near_sdk::env;
use crate::modules::cosmwasm::types::{
    Env, MessageInfo, BlockInfo, ContractInfo, TransactionInfo,
    Timestamp, Addr, Coin, Uint128,
};

/// Get the current environment information in CosmWasm format
pub fn get_cosmwasm_env() -> Env {
    Env {
        block: BlockInfo {
            height: env::block_height(),
            time: Timestamp::from_nanos(env::block_timestamp()),
            chain_id: get_chain_id(),
        },
        transaction: Some(TransactionInfo {
            index: None, // NEAR doesn't have transaction index concept
        }),
        contract: ContractInfo {
            address: Addr::unchecked(env::current_account_id().to_string()),
        },
    }
}

/// Get message info from the current NEAR context
pub fn get_message_info() -> MessageInfo {
    MessageInfo {
        sender: Addr::unchecked(env::predecessor_account_id().to_string()),
        funds: get_attached_funds(),
    }
}

/// Get the chain ID based on the current NEAR network
fn get_chain_id() -> String {
    // Determine chain ID based on current account suffix
    let current_account = env::current_account_id().to_string();
    
    if current_account.ends_with(".near") {
        "proxima-mainnet".to_string()
    } else if current_account.ends_with(".testnet") {
        "proxima-testnet".to_string()
    } else {
        "proxima-local".to_string()
    }
}

/// Convert NEAR attached deposit to CosmWasm coins
fn get_attached_funds() -> Vec<Coin> {
    let attached_deposit = env::attached_deposit();
    let amount_yocto = attached_deposit.as_yoctonear();
    
    if amount_yocto > 0 {
        vec![Coin {
            denom: "near".to_string(),
            amount: Uint128::from(amount_yocto),
        }]
    } else {
        vec![]
    }
}

/// Helper to get current block time as seconds
pub fn get_block_time_seconds() -> u64 {
    env::block_timestamp() / 1_000_000_000
}

/// Helper to get current block time as Timestamp
pub fn get_block_time() -> Timestamp {
    Timestamp::from_nanos(env::block_timestamp())
}

/// Check if the sender matches a specific address
pub fn sender_is(expected: &str) -> bool {
    env::predecessor_account_id().to_string() == expected
}

/// Get the contract's own address
pub fn get_contract_address() -> Addr {
    Addr::unchecked(env::current_account_id().to_string())
}

/// Get the sender's address
pub fn get_sender_address() -> Addr {
    Addr::unchecked(env::predecessor_account_id().to_string())
}

/// Check if any funds are attached to the current call
pub fn has_attached_funds() -> bool {
    env::attached_deposit().as_yoctonear() > 0
}

/// Get attached funds as a specific denomination
pub fn get_attached_funds_as(expected_denom: &str) -> Result<Uint128, String> {
    let funds = get_attached_funds();
    
    if funds.is_empty() {
        return Err("No funds attached".to_string());
    }
    
    if funds.len() > 1 {
        return Err("Multiple denominations not supported".to_string());
    }
    
    let coin = &funds[0];
    if coin.denom != expected_denom {
        return Err(format!(
            "Expected {} but got {}",
            expected_denom, coin.denom
        ));
    }
    
    Ok(coin.amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, AccountId};
    
    fn setup_context(
        predecessor: AccountId,
        current: AccountId,
        attached_deposit: u128,
        block_height: u64,
        block_timestamp: u64,
    ) {
        let context = VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .current_account_id(current)
            .attached_deposit(near_sdk::NearToken::from_yoctonear(attached_deposit))
            .block_height(block_height)
            .block_timestamp(block_timestamp)
            .build();
        testing_env!(context);
    }
    
    #[test]
    fn test_get_cosmwasm_env() {
        setup_context(
            accounts(0),
            "contract.testnet".parse().unwrap(),
            0,
            12345,
            1234567890000000000, // 1234567890 seconds in nanos
        );
        
        let env = get_cosmwasm_env();
        
        assert_eq!(env.block.height, 12345);
        assert_eq!(env.block.time.seconds(), 1234567890);
        assert_eq!(env.block.chain_id, "proxima-testnet");
        assert_eq!(env.contract.address.as_str(), "contract.testnet");
    }
    
    #[test]
    fn test_get_message_info() {
        setup_context(
            "alice.testnet".parse().unwrap(),
            "contract.testnet".parse().unwrap(),
            1_000_000_000_000_000_000_000_000, // 1 NEAR
            100,
            1234567890000000000,
        );
        
        let info = get_message_info();
        
        assert_eq!(info.sender.as_str(), "alice.testnet");
        assert_eq!(info.funds.len(), 1);
        assert_eq!(info.funds[0].denom, "near");
        assert_eq!(info.funds[0].amount.u128(), 1_000_000_000_000_000_000_000_000);
    }
    
    #[test]
    fn test_chain_id_detection() {
        // Test mainnet
        setup_context(
            accounts(0),
            "contract.near".parse().unwrap(),
            0,
            100,
            1234567890000000000,
        );
        assert_eq!(get_chain_id(), "proxima-mainnet");
        
        // Test testnet
        setup_context(
            accounts(0),
            "contract.testnet".parse().unwrap(),
            0,
            100,
            1234567890000000000,
        );
        assert_eq!(get_chain_id(), "proxima-testnet");
        
        // Test local/other
        setup_context(
            accounts(0),
            accounts(1),
            0,
            100,
            1234567890000000000,
        );
        assert_eq!(get_chain_id(), "proxima-local");
    }
    
    #[test]
    fn test_attached_funds_helpers() {
        setup_context(
            accounts(0),
            accounts(1),
            1_000_000_000_000_000_000_000_000, // 1 NEAR
            100,
            1234567890000000000,
        );
        
        assert!(has_attached_funds());
        
        let amount = get_attached_funds_as("near").unwrap();
        assert_eq!(amount.u128(), 1_000_000_000_000_000_000_000_000);
        
        let err = get_attached_funds_as("usdc").unwrap_err();
        assert!(err.contains("Expected usdc but got near"));
    }
}