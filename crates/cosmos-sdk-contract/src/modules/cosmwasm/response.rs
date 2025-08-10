use near_sdk::env;
use base64::{engine::general_purpose::STANDARD, Engine};
use crate::modules::cosmwasm::types::{Response, SubMsg, CosmosMsg, BankMsg, WasmMsg, ReplyOn, Binary};

/// Process a CosmWasm response and translate it to NEAR actions
pub fn process_cosmwasm_response<T>(response: Response<T>) -> Result<String, String> 
where
    T: serde::Serialize,
{
    // Log events as NEAR logs
    for event in &response.events {
        let attributes_str = event.attributes
            .iter()
            .map(|attr| format!("{}={}", attr.key, attr.value))
            .collect::<Vec<_>>()
            .join(" ");
        
        env::log_str(&format!("EVENT[{}] {}", event.ty, attributes_str));
    }
    
    // Log attributes
    for attr in &response.attributes {
        env::log_str(&format!("ATTR[{}]={}", attr.key, attr.value));
    }
    
    // Process sub-messages
    if !response.messages.is_empty() {
        process_sub_messages(response.messages)?;
    }
    
    // Return data if present, otherwise return success message
    match response.data {
        Some(data) => Ok(data.to_base64()),
        None => Ok("{}".to_string()),
    }
}

/// Process sub-messages (cross-contract calls)
fn process_sub_messages<T>(messages: Vec<SubMsg<T>>) -> Result<(), String>
where
    T: serde::Serialize,
{
    for sub_msg in messages {
        env::log_str(&format!(
            "SUBMSG[id={}, reply_on={:?}]",
            sub_msg.id, sub_msg.reply_on
        ));
        
        match sub_msg.msg {
            CosmosMsg::Bank(bank_msg) => process_bank_message(bank_msg, sub_msg.id)?,
            CosmosMsg::Wasm(wasm_msg) => process_wasm_message(wasm_msg, sub_msg.id)?,
            CosmosMsg::Custom(custom) => process_custom_message(custom, sub_msg.id)?,
        }
    }
    
    Ok(())
}

/// Process bank messages (token transfers)
fn process_bank_message(msg: BankMsg, msg_id: u64) -> Result<(), String> {
    match msg {
        BankMsg::Send { to_address, amount } => {
            env::log_str(&format!(
                "BANK_SEND[id={}]: to={}, amount={:?}",
                msg_id, to_address, amount
            ));
            
            // In production, this would:
            // 1. Validate the recipient address
            // 2. Call the Bank module to execute the transfer
            // 3. Handle the result and potentially trigger reply
            
            // For now, just log the action
            for coin in amount {
                env::log_str(&format!(
                    "  Transfer {} {} to {}",
                    coin.amount.u128(), coin.denom, to_address
                ));
            }
        }
        
        BankMsg::Burn { amount } => {
            env::log_str(&format!(
                "BANK_BURN[id={}]: amount={:?}",
                msg_id, amount
            ));
            
            // In production, this would call the Bank module to burn tokens
            
            for coin in amount {
                env::log_str(&format!(
                    "  Burn {} {}",
                    coin.amount.u128(), coin.denom
                ));
            }
        }
    }
    
    Ok(())
}

/// Process WASM messages (contract calls)
fn process_wasm_message(msg: WasmMsg, msg_id: u64) -> Result<(), String> {
    match msg {
        WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        } => {
            env::log_str(&format!(
                "WASM_EXECUTE[id={}]: contract={}, funds={:?}",
                msg_id, contract_addr, funds
            ));
            
            // In production, this would:
            // 1. Create a NEAR cross-contract call
            // 2. Attach the specified funds
            // 3. Execute the contract method
            // 4. Handle callbacks based on reply_on setting
            
            env::log_str(&format!("  Message: {}", STANDARD.encode(msg.as_slice())));
        }
        
        WasmMsg::Instantiate {
            admin,
            code_id,
            msg,
            funds: _,
            label,
        } => {
            env::log_str(&format!(
                "WASM_INSTANTIATE[id={}]: code_id={}, label={}, admin={:?}",
                msg_id, code_id, label, admin
            ));
            
            // In production, this would deploy a new contract instance
            // This is complex in NEAR and would require factory pattern
            
            env::log_str(&format!("  Init message: {}", STANDARD.encode(msg.as_slice())));
        }
        
        WasmMsg::Migrate {
            contract_addr,
            new_code_id,
            msg,
        } => {
            env::log_str(&format!(
                "WASM_MIGRATE[id={}]: contract={}, new_code={}",
                msg_id, contract_addr, new_code_id
            ));
            
            // In production, this would trigger contract migration
            // NEAR has different upgrade semantics than CosmWasm
            
            env::log_str(&format!("  Migrate message: {}", STANDARD.encode(msg.as_slice())));
        }
    }
    
    Ok(())
}

/// Process custom messages (application-specific)
fn process_custom_message<T>(msg: T, msg_id: u64) -> Result<(), String>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string(&msg)
        .map_err(|e| format!("Failed to serialize custom message: {}", e))?;
    
    env::log_str(&format!("CUSTOM_MSG[id={}]: {}", msg_id, json));
    
    // In production, custom messages would be handled based on application logic
    
    Ok(())
}

/// Helper to convert CosmWasm response to JSON string
pub fn response_to_json<T>(response: &Response<T>) -> Result<String, String>
where
    T: serde::Serialize,
{
    // Create a simplified response structure for JSON serialization
    let json_response = serde_json::json!({
        "attributes": response.attributes.iter().map(|attr| {
            serde_json::json!({
                "key": attr.key,
                "value": attr.value,
            })
        }).collect::<Vec<_>>(),
        "events": response.events.iter().map(|event| {
            serde_json::json!({
                "type": event.ty,
                "attributes": event.attributes.iter().map(|attr| {
                    serde_json::json!({
                        "key": attr.key,
                        "value": attr.value,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "messages": response.messages.len(),
        "data": response.data.as_ref().map(|d| d.to_base64()),
    });
    
    serde_json::to_string(&json_response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}

/// Create a reply handler for sub-message responses
pub struct ReplyHandler {
    pub msg_id: u64,
    pub reply_on: ReplyOn,
}

impl ReplyHandler {
    pub fn should_reply(&self, success: bool) -> bool {
        match self.reply_on {
            ReplyOn::Always => true,
            ReplyOn::Success => success,
            ReplyOn::Error => !success,
            ReplyOn::Never => false,
        }
    }
    
    pub fn process_reply(&self, success: bool, data: Option<Binary>) {
        if self.should_reply(success) {
            env::log_str(&format!(
                "REPLY[id={}, success={}, data={}]",
                self.msg_id,
                success,
                data.map(|d| d.to_base64()).unwrap_or_else(|| "null".to_string())
            ));
            
            // In production, this would trigger the contract's reply handler
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::cosmwasm::types::{Event, Coin, Uint128, Empty};
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;
    
    fn setup_context() {
        let context = VMContextBuilder::new()
            .current_account_id("contract.testnet".parse().unwrap())
            .build();
        testing_env!(context);
    }
    
    #[test]
    fn test_process_response_with_events() {
        setup_context();
        
        let response = Response::<Empty>::new()
            .add_attribute("action", "transfer")
            .add_attribute("sender", "alice")
            .add_event(
                Event::new("transfer")
                    .add_attribute("from", "alice")
                    .add_attribute("to", "bob")
                    .add_attribute("amount", "1000")
            );
        
        let result = process_cosmwasm_response(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "{}");
    }
    
    #[test]
    fn test_process_response_with_data() {
        setup_context();
        
        let data = b"test data";
        let response = Response::<Empty>::new()
            .set_data(data.to_vec());
        
        let result = process_cosmwasm_response(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), STANDARD.encode(data));
    }
    
    #[test]
    fn test_process_bank_messages() {
        setup_context();
        
        let msg = SubMsg {
            id: 1,
            msg: CosmosMsg::Bank(BankMsg::Send {
                to_address: "bob.near".to_string(),
                amount: vec![Coin {
                    denom: "near".to_string(),
                    amount: Uint128::new(1000),
                }],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        };
        
        let response = Response::<Empty>::new()
            .add_message(msg);
        
        let result = process_cosmwasm_response(response);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_reply_handler() {
        let handler = ReplyHandler {
            msg_id: 1,
            reply_on: ReplyOn::Success,
        };
        
        assert!(handler.should_reply(true));
        assert!(!handler.should_reply(false));
        
        let handler_always = ReplyHandler {
            msg_id: 2,
            reply_on: ReplyOn::Always,
        };
        
        assert!(handler_always.should_reply(true));
        assert!(handler_always.should_reply(false));
        
        let handler_error = ReplyHandler {
            msg_id: 3,
            reply_on: ReplyOn::Error,
        };
        
        assert!(!handler_error.should_reply(true));
        assert!(handler_error.should_reply(false));
        
        let handler_never = ReplyHandler {
            msg_id: 4,
            reply_on: ReplyOn::Never,
        };
        
        assert!(!handler_never.should_reply(true));
        assert!(!handler_never.should_reply(false));
    }
    
    #[test]
    fn test_response_to_json() {
        let response = Response::<Empty>::new()
            .add_attribute("key1", "value1")
            .add_event(Event::new("test_event"))
            .set_data(b"test".to_vec());
        
        let json = response_to_json(&response).unwrap();
        assert!(json.contains("\"key1\""));
        assert!(json.contains("\"value1\""));
        assert!(json.contains("\"test_event\""));
        assert!(json.contains(&STANDARD.encode(b"test")));
    }
}