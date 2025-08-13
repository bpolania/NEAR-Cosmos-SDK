/// Message Translator for CosmWasm <-> NEAR
/// 
/// This module handles translation between CosmWasm's JSON message format
/// and NEAR's contract call format. It also converts responses between formats.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use near_sdk::{env, AccountId};

/// CosmWasm standard message info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageInfo {
    pub sender: String,
    pub funds: Vec<Coin>,
}

/// CosmWasm coin representation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Coin {
    pub denom: String,
    pub amount: String,
}

/// CosmWasm environment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Env {
    pub block: BlockInfo,
    pub transaction: Option<TransactionInfo>,
    pub contract: ContractInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockInfo {
    pub height: u64,
    pub time: String,  // Nanoseconds as string
    pub chain_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionInfo {
    pub index: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContractInfo {
    pub address: String,
}

/// CosmWasm standard response
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Response {
    pub messages: Vec<SubMsg>,
    pub attributes: Vec<Attribute>,
    pub events: Vec<Event>,
    pub data: Option<String>,
}

/// Sub-message for contract-to-contract calls
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubMsg {
    pub id: u64,
    pub msg: CosmosMsg,
    pub gas_limit: Option<u64>,
    pub reply_on: ReplyOn,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ReplyOn {
    Always,
    Error,
    Success,
    Never,
}

/// CosmWasm message types
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CosmosMsg {
    Bank(BankMsg),
    Custom(Value),
    Wasm(WasmMsg),
    Staking(StakingMsg),
    Distribution(DistributionMsg),
    Gov(GovMsg),
    Ibc(IbcMsg),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BankMsg {
    Send {
        to_address: String,
        amount: Vec<Coin>,
    },
    Burn {
        amount: Vec<Coin>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WasmMsg {
    Execute {
        contract_addr: String,
        msg: String,  // JSON encoded message
        funds: Vec<Coin>,
    },
    Instantiate {
        admin: Option<String>,
        code_id: u64,
        msg: String,  // JSON encoded message
        funds: Vec<Coin>,
        label: String,
    },
    Migrate {
        contract_addr: String,
        new_code_id: u64,
        msg: String,  // JSON encoded message
    },
    UpdateAdmin {
        contract_addr: String,
        admin: String,
    },
    ClearAdmin {
        contract_addr: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StakingMsg;  // Placeholder

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DistributionMsg;  // Placeholder

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GovMsg;  // Placeholder

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IbcMsg;  // Placeholder

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub r#type: String,
    pub attributes: Vec<Attribute>,
}

/// Message translator
pub struct MessageTranslator;

impl MessageTranslator {
    /// Create MessageInfo from NEAR environment
    pub fn create_message_info(sender: &AccountId, attached_deposit: near_sdk::NearToken) -> MessageInfo {
        use crate::address;
        
        // Convert NEAR account to Cosmos address
        let cosmos_sender = address::near_to_cosmos_address(sender, None);
        
        // Convert attached NEAR to Cosmos coins
        let yocto_near = attached_deposit.as_yoctonear();
        let funds = if yocto_near > 0 {
            vec![Coin {
                denom: "unear".to_string(),  // micro-NEAR
                amount: yocto_near.to_string(),
            }]
        } else {
            vec![]
        };
        
        MessageInfo {
            sender: cosmos_sender,
            funds,
        }
    }
    
    /// Create Env from NEAR environment
    pub fn create_env(contract_addr: &str) -> Env {
        Env {
            block: BlockInfo {
                height: env::block_height(),
                time: env::block_timestamp().to_string(),  // Nanoseconds
                chain_id: "near-testnet".to_string(),
            },
            transaction: None,  // NEAR doesn't expose transaction index
            contract: ContractInfo {
                address: contract_addr.to_string(),
            },
        }
    }
    
    /// Convert CosmWasm instantiate args to standard format
    pub fn prepare_instantiate_args(
        msg: &str,
        sender: &AccountId,
        contract_addr: &str,
    ) -> Result<String, String> {
        // Parse the user's message
        let user_msg: Value = serde_json::from_str(msg)
            .map_err(|e| format!("Failed to parse message: {}", e))?;
        
        // Create the full instantiate arguments
        let env = Self::create_env(contract_addr);
        let info = Self::create_message_info(sender, env::attached_deposit());
        
        let args = json!({
            "env": env,
            "info": info,
            "msg": user_msg,
        });
        
        Ok(args.to_string())
    }
    
    /// Convert CosmWasm execute args to standard format
    pub fn prepare_execute_args(
        msg: &str,
        sender: &AccountId,
        contract_addr: &str,
    ) -> Result<String, String> {
        // Parse the user's message
        let user_msg: Value = serde_json::from_str(msg)
            .map_err(|e| format!("Failed to parse message: {}", e))?;
        
        // Create the full execute arguments
        let env = Self::create_env(contract_addr);
        let info = Self::create_message_info(sender, env::attached_deposit());
        
        let args = json!({
            "env": env,
            "info": info,
            "msg": user_msg,
        });
        
        Ok(args.to_string())
    }
    
    /// Convert CosmWasm query args to standard format
    pub fn prepare_query_args(
        msg: &str,
        contract_addr: &str,
    ) -> Result<String, String> {
        // Parse the user's message
        let user_msg: Value = serde_json::from_str(msg)
            .map_err(|e| format!("Failed to parse message: {}", e))?;
        
        // Create the full query arguments
        let env = Self::create_env(contract_addr);
        
        let args = json!({
            "env": env,
            "msg": user_msg,
        });
        
        Ok(args.to_string())
    }
    
    /// Parse CosmWasm response from contract execution
    pub fn parse_response(response_bytes: &[u8]) -> Result<Response, String> {
        // Try to parse as JSON response
        if let Ok(response_str) = std::str::from_utf8(response_bytes) {
            if let Ok(response) = serde_json::from_str::<Response>(response_str) {
                return Ok(response);
            }
        }
        
        // If not JSON, treat as raw data
        Ok(Response {
            data: Some(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, response_bytes)),
            ..Default::default()
        })
    }
    
    /// Convert Response to NEAR return value
    pub fn response_to_near(response: &Response) -> String {
        json!({
            "messages": response.messages,
            "attributes": response.attributes,
            "events": response.events,
            "data": response.data,
        }).to_string()
    }
    
    /// Process sub-messages from a response
    pub fn process_submessages(response: &Response) -> Vec<String> {
        let mut near_calls = Vec::new();
        
        for submsg in &response.messages {
            match &submsg.msg {
                CosmosMsg::Wasm(wasm_msg) => {
                    match wasm_msg {
                        WasmMsg::Execute { contract_addr, msg, funds } => {
                            // Convert to NEAR cross-contract call
                            let call = json!({
                                "receiver_id": contract_addr,
                                "method_name": "execute",
                                "args": msg,
                                "deposit": Self::coins_to_near(funds),
                                "gas": submsg.gas_limit.unwrap_or(30_000_000_000_000u64),
                            });
                            near_calls.push(call.to_string());
                        },
                        WasmMsg::Instantiate { code_id, msg, funds, label, admin } => {
                            let call = json!({
                                "method_name": "instantiate",
                                "args": {
                                    "code_id": code_id,
                                    "msg": msg,
                                    "label": label,
                                    "admin": admin,
                                },
                                "deposit": Self::coins_to_near(funds),
                                "gas": submsg.gas_limit.unwrap_or(30_000_000_000_000u64),
                            });
                            near_calls.push(call.to_string());
                        },
                        _ => {
                            env::log_str(&format!("Unsupported WasmMsg variant: {:?}", wasm_msg));
                        }
                    }
                },
                CosmosMsg::Bank(bank_msg) => {
                    match bank_msg {
                        BankMsg::Send { to_address, amount } => {
                            // Convert to NEAR transfer
                            let call = json!({
                                "receiver_id": to_address,
                                "method_name": "deposit",
                                "args": {},
                                "deposit": Self::coins_to_near(amount),
                                "gas": 5_000_000_000_000u64,
                            });
                            near_calls.push(call.to_string());
                        },
                        _ => {
                            env::log_str(&format!("Unsupported BankMsg variant: {:?}", bank_msg));
                        }
                    }
                },
                _ => {
                    env::log_str(&format!("Unsupported CosmosMsg variant: {:?}", submsg.msg));
                }
            }
        }
        
        near_calls
    }
    
    /// Convert Cosmos coins to NEAR amount (in yoctoNEAR)
    fn coins_to_near(coins: &[Coin]) -> String {
        // Look for NEAR denomination
        for coin in coins {
            if coin.denom == "unear" || coin.denom == "near" {
                if coin.denom == "near" {
                    // Convert NEAR to yoctoNEAR
                    if let Ok(amount) = coin.amount.parse::<u128>() {
                        return (amount * 10u128.pow(24)).to_string();
                    }
                } else {
                    // Already in smallest unit
                    return coin.amount.clone();
                }
            }
        }
        "0".to_string()
    }
    
    /// Convert NEAR amount to Cosmos coins
    pub fn near_to_coins(yocto_near: u128) -> Vec<Coin> {
        if yocto_near > 0 {
            vec![Coin {
                denom: "unear".to_string(),
                amount: yocto_near.to_string(),
            }]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::accounts;
    
    #[test]
    fn test_message_info_creation() {
        let sender = accounts(0);
        let deposit = near_sdk::NearToken::from_yoctonear(1000000);
        let info = MessageTranslator::create_message_info(&sender, deposit);
        
        assert!(info.sender.starts_with("proxima"));
        assert_eq!(info.funds.len(), 1);
        assert_eq!(info.funds[0].denom, "unear");
        assert_eq!(info.funds[0].amount, "1000000");
    }
    
    #[test]
    fn test_env_creation() {
        let env = MessageTranslator::create_env("contract.near");
        
        assert_eq!(env.contract.address, "contract.near");
        assert_eq!(env.block.chain_id, "near-testnet");
    }
    
    #[test]
    fn test_coins_conversion() {
        let coins = vec![
            Coin {
                denom: "near".to_string(),
                amount: "1".to_string(),
            }
        ];
        
        let near_amount = MessageTranslator::coins_to_near(&coins);
        assert_eq!(near_amount, "1000000000000000000000000");
    }
}