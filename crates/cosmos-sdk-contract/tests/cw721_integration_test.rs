use near_sdk::test_utils::{accounts, VMContextBuilder};
use near_sdk::testing_env;
use cosmos_sdk_contract::modules::cosmwasm::{
    CosmWasmContractWrapper, WrapperInitMsg, WrapperExecuteMsg, WrapperQueryMsg,
    types::{Uint128, Binary},
};

/// CW721 NFT Integration Test
/// 
/// This test validates our CosmWasm compatibility layer with a complete CW721 NFT implementation.
/// It demonstrates that our wrapper can handle complex CosmWasm NFT contracts with full functionality.

fn setup_context() {
    let context = VMContextBuilder::new()
        .current_account_id(accounts(0))
        .predecessor_account_id(accounts(1))
        .build();
    testing_env!(context);
}

/// CW721 InstantiateMsg structure
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Cw721InstantiateMsg {
    /// Name of the NFT collection
    name: String,
    /// Symbol of the NFT collection
    symbol: String,
    /// Address who can mint new tokens
    minter: String,
}

/// CW721 ExecuteMsg variants
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Cw721ExecuteMsg {
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft { recipient: String, token_id: String },
    /// Send is a base message to transfer a token to a contract and trigger an action
    SendNft { contract: String, token_id: String, msg: Binary },
    /// Approve allows spender to transfer/send the token from the owner's account
    Approve { spender: String, token_id: String, expires: Option<Expiration> },
    /// Revoke removes an existing approval
    Revoke { spender: String, token_id: String },
    /// ApproveAll allows the operator to transfer/send any token from the owner's account
    ApproveAll { operator: String, expires: Option<Expiration> },
    /// RevokeAll removes an existing ApproveAll authorization
    RevokeAll { operator: String },
    /// Mint a new NFT, can only be called by the contract minter
    Mint { token_id: String, owner: String, token_uri: Option<String>, extension: Option<NftExtension> },
    /// Burn an NFT the sender has access to
    Burn { token_id: String },
    /// Extension message for contract-specific functionality
    Extension { msg: NftExtension },
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Expiration {
    AtHeight(u64),
    AtTime(u64),
    Never {},
}

/// CW721 QueryMsg variants
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Cw721QueryMsg {
    /// Return the owner of the given token, error if token does not exist
    OwnerOf { token_id: String, include_expired: Option<bool> },
    /// Return operator that can access all of the owner's tokens
    Approval { token_id: String, spender: String, include_expired: Option<bool> },
    /// Return approvals that a token has
    Approvals { token_id: String, include_expired: Option<bool> },
    /// Return approval of all operators that can access all of the owner's tokens
    AllOperators { owner: String, include_expired: Option<bool>, start_after: Option<String>, limit: Option<u32> },
    /// Total number of tokens issued
    NumTokens {},
    /// Return contract info
    ContractInfo {},
    /// Return metadata about one particular token
    NftInfo { token_id: String },
    /// Return metadata for all tokens
    AllNftInfo { token_id: String, include_expired: Option<bool> },
    /// With MetaData extension: returns metadata about the contract itself
    GetCollectionInfo {},
    /// Return the tokens owned by the given address
    Tokens { owner: String, start_after: Option<String>, limit: Option<u32> },
    /// Enumerate all tokens
    AllTokens { start_after: Option<String>, limit: Option<u32> },
    /// Return the minter
    Minter {},
}

/// NFT extension for custom metadata
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct NftExtension {
    name: Option<String>,
    description: Option<String>,
    image: Option<String>,
    attributes: Option<Vec<Trait>>,
    background_color: Option<String>,
    animation_url: Option<String>,
    youtube_url: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Trait {
    display_type: Option<String>,
    trait_type: String,
    value: String,
}

/// Response types
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct OwnerOfResponse {
    owner: String,
    approvals: Vec<Approval>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Approval {
    spender: String,
    expires: Expiration,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ContractInfoResponse {
    name: String,
    symbol: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct NftInfoResponse {
    token_uri: Option<String>,
    extension: Option<NftExtension>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct NumTokensResponse {
    count: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct MinterResponse {
    minter: String,
}

#[test]
fn test_cw721_nft_instantiation() {
    setup_context();
    
    // Create CW721 instantiate message
    let cw721_init = Cw721InstantiateMsg {
        name: "Proxima Test NFTs".to_string(),
        symbol: "PNFT".to_string(),
        minter: accounts(1).to_string(),
    };
    
    // Create wrapper init message
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    // Initialize the contract wrapper
    let contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Verify initialization
    assert!(contract.get_contract_info().initialized);
    assert_eq!(contract.get_contract_info().version, "1.0.0");
    assert_eq!(contract.get_contract_info().admin, Some(accounts(1).to_string()));
}

#[test]
fn test_cw721_contract_info_query() {
    setup_context();
    
    // Initialize contract
    let cw721_init = Cw721InstantiateMsg {
        name: "Info Test NFTs".to_string(),
        symbol: "INFO".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Query contract info
    let query_msg = Cw721QueryMsg::ContractInfo {};
    let wrapper_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&query_msg).unwrap(),
    };
    
    let response = contract.query(wrapper_query);
    assert!(response.success);
    assert!(response.data.is_some());
}

#[test]
fn test_cw721_mint_nft() {
    setup_context();
    
    // Initialize contract
    let cw721_init = Cw721InstantiateMsg {
        name: "Mint Test NFTs".to_string(),
        symbol: "MINT".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Mint NFT with metadata
    let extension = NftExtension {
        name: Some("Cool Dragon".to_string()),
        description: Some("A very cool dragon NFT for testing".to_string()),
        image: Some("https://example.com/dragon.png".to_string()),
        attributes: Some(vec![
            Trait {
                display_type: None,
                trait_type: "Element".to_string(),
                value: "Fire".to_string(),
            },
            Trait {
                display_type: Some("number".to_string()),
                trait_type: "Power Level".to_string(),
                value: "9000".to_string(),
            },
        ]),
        background_color: Some("#FF0000".to_string()),
        animation_url: None,
        youtube_url: None,
    };
    
    let execute_msg = Cw721ExecuteMsg::Mint {
        token_id: "dragon_001".to_string(),
        owner: accounts(2).to_string(),
        token_uri: Some("https://example.com/metadata/dragon_001.json".to_string()),
        extension: Some(extension),
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&execute_msg).unwrap(),
    };
    
    let response = contract.execute(wrapper_execute);
    assert!(response.success);
    assert!(response.data.is_some());
}

#[test]
fn test_cw721_owner_query() {
    setup_context();
    
    // Initialize and mint an NFT
    let cw721_init = Cw721InstantiateMsg {
        name: "Owner Test NFTs".to_string(),
        symbol: "OWN".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Mint an NFT
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "test_001".to_string(),
        owner: accounts(2).to_string(),
        token_uri: None,
        extension: None,
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&mint_msg).unwrap(),
    };
    
    contract.execute(wrapper_execute);
    
    // Query owner
    let query_msg = Cw721QueryMsg::OwnerOf {
        token_id: "test_001".to_string(),
        include_expired: Some(false),
    };
    
    let wrapper_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&query_msg).unwrap(),
    };
    
    let response = contract.query(wrapper_query);
    assert!(response.success);
    assert!(response.data.is_some());
}

#[test]
fn test_cw721_transfer_nft() {
    setup_context();
    
    // Initialize contract
    let cw721_init = Cw721InstantiateMsg {
        name: "Transfer Test NFTs".to_string(),
        symbol: "TRANS".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Mint NFT
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "transfer_001".to_string(),
        owner: accounts(2).to_string(),
        token_uri: Some("https://example.com/metadata/transfer_001.json".to_string()),
        extension: None,
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&mint_msg).unwrap(),
    };
    
    contract.execute(wrapper_execute);
    
    // Transfer NFT
    let transfer_msg = Cw721ExecuteMsg::TransferNft {
        recipient: accounts(3).to_string(),
        token_id: "transfer_001".to_string(),
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&transfer_msg).unwrap(),
    };
    
    let response = contract.execute(wrapper_execute);
    assert!(response.success);
    assert!(response.data.is_some());
}

#[test]
fn test_cw721_approve_operations() {
    setup_context();
    
    // Initialize contract
    let cw721_init = Cw721InstantiateMsg {
        name: "Approve Test NFTs".to_string(),
        symbol: "APPR".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Mint NFT
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "approve_001".to_string(),
        owner: accounts(2).to_string(),
        token_uri: None,
        extension: None,
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&mint_msg).unwrap(),
    };
    
    contract.execute(wrapper_execute);
    
    // Approve spender
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: accounts(3).to_string(),
        token_id: "approve_001".to_string(),
        expires: Some(Expiration::Never {}),
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&approve_msg).unwrap(),
    };
    
    let response = contract.execute(wrapper_execute);
    assert!(response.success);
    
    // Query approval
    let query_msg = Cw721QueryMsg::Approval {
        token_id: "approve_001".to_string(),
        spender: accounts(3).to_string(),
        include_expired: Some(false),
    };
    
    let wrapper_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&query_msg).unwrap(),
    };
    
    let query_response = contract.query(wrapper_query);
    assert!(query_response.success);
    assert!(query_response.data.is_some());
}

#[test]
fn test_cw721_approve_all_operations() {
    setup_context();
    
    // Initialize contract
    let cw721_init = Cw721InstantiateMsg {
        name: "Approve All Test NFTs".to_string(),
        symbol: "APALL".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Approve all tokens for operator
    let approve_all_msg = Cw721ExecuteMsg::ApproveAll {
        operator: accounts(3).to_string(),
        expires: Some(Expiration::Never {}),
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&approve_all_msg).unwrap(),
    };
    
    let response = contract.execute(wrapper_execute);
    assert!(response.success);
    
    // Query all operators
    let query_msg = Cw721QueryMsg::AllOperators {
        owner: accounts(2).to_string(),
        include_expired: Some(false),
        start_after: None,
        limit: None,
    };
    
    let wrapper_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&query_msg).unwrap(),
    };
    
    let query_response = contract.query(wrapper_query);
    assert!(query_response.success);
    assert!(query_response.data.is_some());
}

#[test]
fn test_cw721_burn_nft() {
    setup_context();
    
    // Initialize contract
    let cw721_init = Cw721InstantiateMsg {
        name: "Burn Test NFTs".to_string(),
        symbol: "BURN".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Mint NFT
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "burn_001".to_string(),
        owner: accounts(2).to_string(),
        token_uri: None,
        extension: None,
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&mint_msg).unwrap(),
    };
    
    contract.execute(wrapper_execute);
    
    // Burn NFT
    let burn_msg = Cw721ExecuteMsg::Burn {
        token_id: "burn_001".to_string(),
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&burn_msg).unwrap(),
    };
    
    let response = contract.execute(wrapper_execute);
    assert!(response.success);
    assert!(response.data.is_some());
}

#[test]
fn test_cw721_send_nft_with_message() {
    setup_context();
    
    // Initialize contract
    let cw721_init = Cw721InstantiateMsg {
        name: "Send Test NFTs".to_string(),
        symbol: "SEND".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Mint NFT
    let mint_msg = Cw721ExecuteMsg::Mint {
        token_id: "send_001".to_string(),
        owner: accounts(2).to_string(),
        token_uri: None,
        extension: None,
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&mint_msg).unwrap(),
    };
    
    contract.execute(wrapper_execute);
    
    // Send NFT to contract with message
    let send_msg = serde_json::json!({"action": "stake_nft", "duration": 30});
    let send_nft_msg = Cw721ExecuteMsg::SendNft {
        contract: accounts(4).to_string(), // NFT staking contract
        token_id: "send_001".to_string(),
        msg: Binary::from(send_msg.to_string().as_bytes()),
    };
    
    let wrapper_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&send_nft_msg).unwrap(),
    };
    
    let response = contract.execute(wrapper_execute);
    assert!(response.success);
    assert!(response.data.is_some());
}

#[test]
fn test_cw721_comprehensive_workflow() {
    setup_context();
    
    // Initialize a comprehensive CW721 NFT collection
    let cw721_init = Cw721InstantiateMsg {
        name: "Proxima Dragons Collection".to_string(),
        symbol: "DRAGN".to_string(),
        minter: accounts(1).to_string(),
    };
    
    let wrapper_init = WrapperInitMsg {
        admin: Some(accounts(1).to_string()),
        version: Some("1.0.0".to_string()),
        contract_msg: serde_json::to_string(&cw721_init).unwrap(),
    };
    
    let mut contract = CosmWasmContractWrapper::new(wrapper_init);
    
    // Test 1: Query initial contract info
    let contract_info_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&Cw721QueryMsg::ContractInfo {}).unwrap(),
    };
    let response = contract.query(contract_info_query);
    assert!(response.success);
    
    // Test 2: Query initial token count (should be 0)
    let num_tokens_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&Cw721QueryMsg::NumTokens {}).unwrap(),
    };
    let response = contract.query(num_tokens_query);
    assert!(response.success);
    
    // Test 3: Mint first NFT with rich metadata
    let dragon_1_extension = NftExtension {
        name: Some("Ancient Fire Dragon".to_string()),
        description: Some("A legendary dragon that has lived for thousands of years, master of fire magic".to_string()),
        image: Some("https://proxima.example.com/dragons/fire_dragon.png".to_string()),
        attributes: Some(vec![
            Trait {
                display_type: None,
                trait_type: "Element".to_string(),
                value: "Fire".to_string(),
            },
            Trait {
                display_type: Some("number".to_string()),
                trait_type: "Power Level".to_string(),
                value: "9500".to_string(),
            },
            Trait {
                display_type: None,
                trait_type: "Rarity".to_string(),
                value: "Legendary".to_string(),
            },
            Trait {
                display_type: Some("number".to_string()),
                trait_type: "Age".to_string(),
                value: "3000".to_string(),
            },
        ]),
        background_color: Some("#FF4500".to_string()),
        animation_url: Some("https://proxima.example.com/dragons/fire_dragon_anim.mp4".to_string()),
        youtube_url: None,
    };
    
    let mint_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&Cw721ExecuteMsg::Mint {
            token_id: "dragon_fire_001".to_string(),
            owner: accounts(2).to_string(),
            token_uri: Some("https://proxima.example.com/metadata/dragon_fire_001.json".to_string()),
            extension: Some(dragon_1_extension),
        }).unwrap(),
    };
    let response = contract.execute(mint_execute);
    assert!(response.success);
    
    // Test 4: Mint second NFT with different attributes
    let dragon_2_extension = NftExtension {
        name: Some("Mystic Ice Dragon".to_string()),
        description: Some("A rare ice dragon with the power to freeze entire kingdoms".to_string()),
        image: Some("https://proxima.example.com/dragons/ice_dragon.png".to_string()),
        attributes: Some(vec![
            Trait {
                display_type: None,
                trait_type: "Element".to_string(),
                value: "Ice".to_string(),
            },
            Trait {
                display_type: Some("number".to_string()),
                trait_type: "Power Level".to_string(),
                value: "8800".to_string(),
            },
            Trait {
                display_type: None,
                trait_type: "Rarity".to_string(),
                value: "Epic".to_string(),
            },
        ]),
        background_color: Some("#00BFFF".to_string()),
        animation_url: None,
        youtube_url: None,
    };
    
    let mint_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&Cw721ExecuteMsg::Mint {
            token_id: "dragon_ice_001".to_string(),
            owner: accounts(3).to_string(),
            token_uri: Some("https://proxima.example.com/metadata/dragon_ice_001.json".to_string()),
            extension: Some(dragon_2_extension),
        }).unwrap(),
    };
    let response = contract.execute(mint_execute);
    assert!(response.success);
    
    // Test 5: Query NFT info
    let nft_info_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&Cw721QueryMsg::NftInfo {
            token_id: "dragon_fire_001".to_string(),
        }).unwrap(),
    };
    let response = contract.query(nft_info_query);
    assert!(response.success);
    
    // Test 6: Query all NFT info
    let all_nft_info_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&Cw721QueryMsg::AllNftInfo {
            token_id: "dragon_fire_001".to_string(),
            include_expired: Some(false),
        }).unwrap(),
    };
    let response = contract.query(all_nft_info_query);
    assert!(response.success);
    
    // Test 7: Set up approval for marketplace
    let approve_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&Cw721ExecuteMsg::Approve {
            spender: accounts(0).to_string(), // Marketplace contract
            token_id: "dragon_fire_001".to_string(),
            expires: Some(Expiration::AtHeight(1000000)),
        }).unwrap(),
    };
    let response = contract.execute(approve_execute);
    assert!(response.success);
    
    // Test 8: Transfer NFT
    let transfer_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&Cw721ExecuteMsg::TransferNft {
            recipient: accounts(4).to_string(),
            token_id: "dragon_ice_001".to_string(),
        }).unwrap(),
    };
    let response = contract.execute(transfer_execute);
    assert!(response.success);
    
    // Test 9: Query tokens owned by account
    let tokens_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&Cw721QueryMsg::Tokens {
            owner: accounts(2).to_string(),
            start_after: None,
            limit: Some(10),
        }).unwrap(),
    };
    let response = contract.query(tokens_query);
    assert!(response.success);
    
    // Test 10: Query all tokens
    let all_tokens_query = WrapperQueryMsg {
        contract_msg: serde_json::to_string(&Cw721QueryMsg::AllTokens {
            start_after: None,
            limit: Some(100),
        }).unwrap(),
    };
    let response = contract.query(all_tokens_query);
    assert!(response.success);
    
    // Test 11: Send NFT to staking contract with message
    let stake_msg = serde_json::json!({
        "action": "stake",
        "duration_days": 90,
        "reward_token": "NEAR",
        "auto_compound": true
    });
    
    let send_execute = WrapperExecuteMsg {
        contract_msg: serde_json::to_string(&Cw721ExecuteMsg::SendNft {
            contract: accounts(5).to_string(), // Staking contract
            token_id: "dragon_fire_001".to_string(),
            msg: Binary::from(stake_msg.to_string().as_bytes()),
        }).unwrap(),
    };
    let response = contract.execute(send_execute);
    assert!(response.success);
    
    // Verify contract state is maintained
    let final_info = contract.get_contract_info();
    assert!(final_info.initialized);
    assert_eq!(final_info.version, "1.0.0");
}