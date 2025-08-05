#[cfg(test)]
mod cosmwasm_compatibility_tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, AccountId};
    use cosmos_sdk_contract::modules::cosmwasm::{
        types::{*, self as cosmwasm_std},
        storage::CosmWasmStorage,
        api::CosmWasmApi,
        deps::{CosmWasmDeps, CosmWasmDepsMut},
        env::{get_cosmwasm_env, get_message_info},
        response::process_cosmwasm_response,
    };
    
    /// Example CosmWasm contract logic (counter)
    pub struct CounterContract;
    
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct InstantiateMsg {
        pub count: u128,
    }
    
    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum ExecuteMsg {
        Increment {},
        Reset { count: u128 },
    }
    
    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum QueryMsg {
        GetCount {},
    }
    
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct CountResponse {
        pub count: u128,
    }
    
    const COUNT_KEY: &[u8] = b"count";
    
    impl CounterContract {
        pub fn instantiate(
            deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            msg: InstantiateMsg,
        ) -> StdResult<Response> {
            // Store initial count
            deps.storage.set(COUNT_KEY, &msg.count.to_le_bytes());
            
            Ok(Response::new()
                .add_attribute("action", "instantiate")
                .add_attribute("count", msg.count.to_string()))
        }
        
        pub fn execute(
            deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            msg: ExecuteMsg,
        ) -> StdResult<Response> {
            match msg {
                ExecuteMsg::Increment {} => {
                    // Load current count
                    let count_bytes = deps.storage.get(COUNT_KEY)
                        .ok_or_else(|| StdError::not_found("count"))?;
                    
                    let mut count_array = [0u8; 16];
                    count_array.copy_from_slice(&count_bytes[..16]);
                    let mut count = u128::from_le_bytes(count_array);
                    
                    // Increment
                    count += 1;
                    
                    // Save new count
                    deps.storage.set(COUNT_KEY, &count.to_le_bytes());
                    
                    Ok(Response::new()
                        .add_attribute("action", "increment")
                        .add_attribute("count", count.to_string()))
                }
                
                ExecuteMsg::Reset { count } => {
                    // Save new count
                    deps.storage.set(COUNT_KEY, &count.to_le_bytes());
                    
                    Ok(Response::new()
                        .add_attribute("action", "reset")
                        .add_attribute("count", count.to_string()))
                }
            }
        }
        
        pub fn query(
            deps: Deps,
            _env: Env,
            msg: QueryMsg,
        ) -> StdResult<Binary> {
            match msg {
                QueryMsg::GetCount {} => {
                    // Load current count
                    let count_bytes = deps.storage.get(COUNT_KEY)
                        .ok_or_else(|| StdError::not_found("count"))?;
                    
                    let mut count_array = [0u8; 16];
                    count_array.copy_from_slice(&count_bytes[..16]);
                    let count = u128::from_le_bytes(count_array);
                    
                    let response = CountResponse { count };
                    let binary = serde_json::to_vec(&response)
                        .map_err(|e| StdError::serialize_err("CountResponse", e.to_string()))?;
                    
                    Ok(Binary::from(binary))
                }
            }
        }
    }
    
    fn setup_test_context(predecessor: AccountId) {
        let context = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(predecessor)
            .attached_deposit(0)
            .build();
        testing_env!(context);
    }
    
    #[test]
    fn test_cosmwasm_counter_contract() {
        setup_test_context(accounts(1));
        
        // Create CosmWasm compatibility components
        let mut storage = CosmWasmStorage::new();
        let api = CosmWasmApi::new();
        
        // Test instantiation
        {
            let mut deps_mut = CosmWasmDepsMut::new(&mut storage, &api);
            let env = get_cosmwasm_env();
            let info = get_message_info();
            let msg = InstantiateMsg { count: 100 };
            
            let result = CounterContract::instantiate(
                deps_mut.as_deps_mut(),
                env,
                info,
                msg,
            );
            
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.attributes.len(), 2);
            assert_eq!(response.attributes[0].key, "action");
            assert_eq!(response.attributes[0].value, "instantiate");
            assert_eq!(response.attributes[1].key, "count");
            assert_eq!(response.attributes[1].value, "100");
        }
        
        // Test query
        {
            let deps = CosmWasmDeps::new(&storage, &api);
            let env = get_cosmwasm_env();
            let msg = QueryMsg::GetCount {};
            
            let result = CounterContract::query(
                deps.as_deps(),
                env,
                msg,
            );
            
            assert!(result.is_ok());
            let binary = result.unwrap();
            let response: CountResponse = serde_json::from_slice(binary.as_slice()).unwrap();
            assert_eq!(response.count, 100);
        }
        
        // Test increment
        {
            let mut deps_mut = CosmWasmDepsMut::new(&mut storage, &api);
            let env = get_cosmwasm_env();
            let info = get_message_info();
            let msg = ExecuteMsg::Increment {};
            
            let result = CounterContract::execute(
                deps_mut.as_deps_mut(),
                env,
                info,
                msg,
            );
            
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.attributes[1].value, "101");
        }
        
        // Test reset
        {
            let mut deps_mut = CosmWasmDepsMut::new(&mut storage, &api);
            let env = get_cosmwasm_env();
            let info = get_message_info();
            let msg = ExecuteMsg::Reset { count: 200 };
            
            let result = CounterContract::execute(
                deps_mut.as_deps_mut(),
                env,
                info,
                msg,
            );
            
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.attributes[1].value, "200");
        }
        
        // Verify final count
        {
            let deps = CosmWasmDeps::new(&storage, &api);
            let env = get_cosmwasm_env();
            let msg = QueryMsg::GetCount {};
            
            let result = CounterContract::query(
                deps.as_deps(),
                env,
                msg,
            );
            
            assert!(result.is_ok());
            let binary = result.unwrap();
            let response: CountResponse = serde_json::from_slice(binary.as_slice()).unwrap();
            assert_eq!(response.count, 200);
        }
    }
    
    #[test]
    fn test_cosmwasm_response_processing() {
        setup_test_context(accounts(1));
        
        // Test response with events and attributes
        let response = Response::new()
            .add_attribute("action", "transfer")
            .add_event(
                Event::new("wasm")
                    .add_attribute("_contract_address", "contract.near")
                    .add_attribute("action", "transfer")
                    .add_attribute("from", "alice")
                    .add_attribute("to", "bob")
                    .add_attribute("amount", "1000")
            )
            .add_message(SubMsg {
                id: 1,
                msg: CosmosMsg::Bank(BankMsg::Send {
                    to_address: "bob.near".to_string(),
                    amount: vec![Coin {
                        denom: "near".to_string(),
                        amount: Uint128::new(1000),
                    }],
                }),
                gas_limit: Some(100_000),
                reply_on: ReplyOn::Success,
            });
        
        let result = process_cosmwasm_response(response);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_storage_range_queries() {
        setup_test_context(accounts(1));
        
        let mut storage = CosmWasmStorage::new();
        
        // Set up test data
        storage.set(b"user:alice", b"100");
        storage.set(b"user:bob", b"200");
        storage.set(b"user:charlie", b"300");
        storage.set(b"config:fee", b"10");
        
        // Test prefix range
        let users: Vec<_> = storage
            .prefix_range(b"user:", Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].0, b"user:alice");
        assert_eq!(users[1].0, b"user:bob");
        assert_eq!(users[2].0, b"user:charlie");
        
        // Test bounded range
        let subset: Vec<_> = storage
            .range(Some(b"user:alice"), Some(b"user:charlie"), Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert_eq!(subset.len(), 2);
        assert_eq!(subset[0].0, b"user:alice");
        assert_eq!(subset[1].0, b"user:bob");
    }
    
    #[test]
    fn test_address_validation() {
        let api = CosmWasmApi::new();
        
        // Test NEAR addresses
        assert!(api.addr_validate("alice.near").is_ok());
        assert!(api.addr_validate("alice.testnet").is_ok());
        
        // Test Cosmos addresses
        assert!(api.addr_validate("cosmos1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzv7xu").is_ok());
        
        // Test Proxima addresses
        assert!(api.addr_validate("proxima1qypqxpq9qcrsszg2pvxq6rs0zqg3yyc5lzv7xu2").is_ok());
        
        // Test invalid addresses
        assert!(api.addr_validate("invalid").is_err());
        assert!(api.addr_validate("").is_err());
    }
}