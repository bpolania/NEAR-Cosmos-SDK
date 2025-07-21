use anyhow::Result;
use near_workspaces::{types::NearToken, Account, Contract, Worker};
use serde_json::json;

const COSMOS_SDK_NEAR_WASM: &[u8] = include_bytes!("../target/near/cosmos_sdk_near.wasm");

/// Helper function to deploy the Cosmos SDK NEAR contract
async fn deploy_cosmos_contract(worker: &Worker<near_workspaces::network::Sandbox>) -> Result<Contract> {
    let contract = worker.dev_deploy(COSMOS_SDK_NEAR_WASM).await?;
    
    // Initialize the contract
    contract
        .call("new")
        .args_json(json!({}))
        .transact()
        .await?
        .into_result()?;
    
    Ok(contract)
}

/// Helper function to create a test account
async fn create_test_account(worker: &Worker<near_workspaces::network::Sandbox>, name: &str) -> Result<Account> {
    let root = worker.root_account()?;
    let account = root
        .create_subaccount(name)
        .initial_balance(NearToken::from_near(10))
        .transact()
        .await?
        .into_result()?;
    
    Ok(account)
}

/// Helper function to setup a basic IBC client for testing
async fn setup_ibc_client(contract: &Contract, caller: &Account) -> Result<String> {
    let header = json!({
        "signed_header": {
            "header": {
                "version": { "block": 11, "app": 0 },
                "chain_id": "test-chain-1",
                "height": 100,
                "time": 1640995200,
                "last_block_id": {
                    "hash": [18, 52, 86, 120, 144, 171, 205, 239],
                    "part_set_header": { "total": 1, "hash": [171, 205, 239, 18, 52, 86, 120, 144] }
                },
                "last_commit_hash": [17, 17, 17, 17, 17, 17, 17, 17],
                "data_hash": [34, 34, 34, 34, 34, 34, 34, 34],
                "validators_hash": [51, 51, 51, 51, 51, 51, 51, 51],
                "next_validators_hash": [68, 68, 68, 68, 68, 68, 68, 68],
                "consensus_hash": [85, 85, 85, 85, 85, 85, 85, 85],
                "app_hash": [102, 102, 102, 102, 102, 102, 102, 102],
                "last_results_hash": [119, 119, 119, 119, 119, 119, 119, 119],
                "evidence_hash": [136, 136, 136, 136, 136, 136, 136, 136],
                "proposer_address": [153, 153, 153, 153, 153, 153, 153, 153]
            },
            "commit": {
                "height": 100,
                "round": 0,
                "block_id": {
                    "hash": [18, 52, 86, 120, 144, 171, 205, 239],
                    "part_set_header": { "total": 1, "hash": [171, 205, 239, 18, 52, 86, 120, 144] }
                },
                "signatures": [
                    {
                        "block_id_flag": 2,
                        "validator_address": [1, 2, 3, 4, 5, 6, 7, 8],
                        "timestamp": 1640995200,
                        "signature": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64]
                    }
                ]
            }
        },
        "validator_set": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000000
        },
        "trusted_height": { "revision_number": 1, "revision_height": 50 },
        "trusted_validators": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000000
        }
    });

    let result = caller
        .call(contract.id(), "ibc_create_client")
        .args_json(json!({
            "chain_id": "test-chain-1",
            "trust_period": 1209600,  // 14 days
            "unbonding_period": 2419200,  // 28 days
            "max_clock_drift": 3600,  // 1 hour
            "initial_header": header
        }))
        .transact()
        .await?
        .into_result()?;

    let client_id: String = result.json()?;
    Ok(client_id)
}

/// Helper function to setup an IBC connection for testing
async fn setup_ibc_connection(contract: &Contract, caller: &Account, client_id: &str) -> Result<String> {
    let result = caller
        .call(contract.id(), "ibc_conn_open_init")
        .args_json(json!({
            "client_id": client_id,
            "counterparty_client_id": "07-tendermint-1",
            "counterparty_prefix": [105, 98, 99], // "ibc"
            "version": {
                "identifier": "1",
                "features": ["ORDER_ORDERED", "ORDER_UNORDERED"]
            },
            "delay_period": 0
        }))
        .transact()
        .await?
        .into_result()?;

    let connection_id: String = result.json()?;
    Ok(connection_id)
}

#[tokio::test]
async fn test_channel_open_init() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup prerequisites
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;

    // Test ChanOpenInit
    let result = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0, // Unordered
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?;

    let channel_id: String = result.json()?;
    assert_eq!(channel_id, "channel-0");

    // Verify channel state
    let channel = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(channel.is_some());
    let channel = channel.unwrap();
    assert_eq!(channel["state"], "Init");
    assert_eq!(channel["ordering"], "Unordered");
    assert_eq!(channel["version"], "ics20-1");

    println!("✅ Channel Init test passed - Channel ID: {}", channel_id);
    Ok(())
}

#[tokio::test]
async fn test_channel_open_try() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup prerequisites
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;

    // Test ChanOpenTry
    let result = alice
        .call(contract.id(), "ibc_chan_open_try")
        .args_json(json!({
            "port_id": "transfer",
            "previous_channel_id": null,
            "order": 1, // Ordered
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "counterparty_channel_id": "channel-0",
            "version": "ics20-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4], // Mock proof
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    let channel_id: String = result.json()?;
    assert_eq!(channel_id, "channel-0");

    // Verify channel state
    let channel = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(channel.is_some());
    let channel = channel.unwrap();
    assert_eq!(channel["state"], "TryOpen");
    assert_eq!(channel["ordering"], "Ordered");
    assert_eq!(channel["counterparty"]["channel_id"], "channel-0");

    println!("✅ Channel Try test passed - Channel ID: {}", channel_id);
    Ok(())
}

#[tokio::test]
async fn test_channel_open_ack() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup prerequisites and initial channel
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;
    
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    // Test ChanOpenAck
    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "counterparty_channel_id": "channel-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4], // Mock proof
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Verify channel state
    let channel = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(channel.is_some());
    let channel = channel.unwrap();
    assert_eq!(channel["state"], "Open");
    assert_eq!(channel["counterparty"]["channel_id"], "channel-1");

    println!("✅ Channel Ack test passed - Channel is now Open");
    Ok(())
}

#[tokio::test]
async fn test_channel_open_confirm() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup prerequisites and TryOpen channel
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;
    
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_try")
        .args_json(json!({
            "port_id": "transfer",
            "previous_channel_id": null,
            "order": 1,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "counterparty_channel_id": "channel-0",
            "version": "ics20-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    // Test ChanOpenConfirm
    alice
        .call(contract.id(), "ibc_chan_open_confirm")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "channel_proof": [1, 2, 3, 4], // Mock proof
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Verify channel state
    let channel = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(channel.is_some());
    let channel = channel.unwrap();
    assert_eq!(channel["state"], "Open");

    println!("✅ Channel Confirm test passed - Channel is now Open");
    Ok(())
}

#[tokio::test]
async fn test_complete_channel_handshake() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup prerequisites
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;

    // Complete handshake: Init -> Try -> Ack -> Confirm
    
    // Step 1: ChanOpenInit
    let channel_id_a = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id.clone()],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    // Step 2: ChanOpenTry (on "counterparty")
    let channel_id_b = alice
        .call(contract.id(), "ibc_chan_open_try")
        .args_json(json!({
            "port_id": "transfer",
            "previous_channel_id": null,
            "order": 0,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "counterparty_channel_id": channel_id_a,
            "version": "ics20-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    // Step 3: ChanOpenAck
    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id_a,
            "counterparty_channel_id": channel_id_b,
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Step 4: ChanOpenConfirm
    alice
        .call(contract.id(), "ibc_chan_open_confirm")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id_b,
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Verify both channels are Open
    let channel_a = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id_a
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?
        .unwrap();

    let channel_b = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id_b
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?
        .unwrap();

    assert_eq!(channel_a["state"], "Open");
    assert_eq!(channel_b["state"], "Open");
    assert_eq!(channel_a["counterparty"]["channel_id"], channel_id_b);
    assert_eq!(channel_b["counterparty"]["channel_id"], channel_id_a);

    println!("✅ Complete handshake test passed - Both channels are Open");
    println!("   Channel A: {} <-> Channel B: {}", channel_id_a, channel_id_b);
    Ok(())
}

#[tokio::test]
async fn test_send_packet() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup open channel
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;
    
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    // Ack to make channel Open
    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "counterparty_channel_id": "channel-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Test SendPacket
    let result = alice
        .call(contract.id(), "ibc_send_packet")
        .args_json(json!({
            "source_port": "transfer",
            "source_channel": channel_id,
            "timeout_height_revision": 1,
            "timeout_height_value": 1000,
            "timeout_timestamp": 0, // No timestamp timeout
            "data": [72, 101, 108, 108, 111] // "Hello"
        }))
        .transact()
        .await?
        .into_result()?;

    let sequence: u64 = result.json()?;
    assert_eq!(sequence, 1);

    // Verify next sequence incremented
    let next_send = alice
        .call(contract.id(), "ibc_get_next_sequence_send")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<u64>()?;

    assert_eq!(next_send, 2);

    // Verify packet commitment exists
    let commitment = alice
        .call(contract.id(), "ibc_get_packet_commitment")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "sequence": sequence
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(commitment.is_some());

    println!("✅ SendPacket test passed - Sequence: {}", sequence);
    Ok(())
}

#[tokio::test]
async fn test_recv_packet() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup open channel
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;
    
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "counterparty_channel_id": "channel-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Test RecvPacket
    alice
        .call(contract.id(), "ibc_recv_packet")
        .args_json(json!({
            "sequence": 1,
            "source_port": "transfer",
            "source_channel": "channel-1",
            "destination_port": "transfer",
            "destination_channel": channel_id,
            "data": [72, 101, 108, 108, 111], // "Hello"
            "timeout_height_revision": 1,
            "timeout_height_value": 1000,
            "timeout_timestamp": 0,
            "packet_proof": [1, 2, 3, 4], // Mock proof
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Verify packet receipt exists
    let receipt = alice
        .call(contract.id(), "ibc_get_packet_receipt")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "sequence": 1
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(receipt.is_some());
    assert_eq!(receipt.unwrap()["sequence"], 1);

    println!("✅ RecvPacket test passed - Packet received and receipt stored");
    Ok(())
}

#[tokio::test]
async fn test_acknowledge_packet() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup open channel and send packet
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;
    
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "counterparty_channel_id": "channel-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Send a packet first
    let sequence = alice
        .call(contract.id(), "ibc_send_packet")
        .args_json(json!({
            "source_port": "transfer",
            "source_channel": channel_id,
            "timeout_height_revision": 1,
            "timeout_height_value": 1000,
            "timeout_timestamp": 0,
            "data": [72, 101, 108, 108, 111] // "Hello"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<u64>()?;

    // Test AcknowledgePacket
    alice
        .call(contract.id(), "ibc_acknowledge_packet")
        .args_json(json!({
            "sequence": sequence,
            "source_port": "transfer",
            "source_channel": channel_id,
            "destination_port": "transfer",
            "destination_channel": "channel-1",
            "data": [72, 101, 108, 108, 111], // "Hello"
            "timeout_height_revision": 1,
            "timeout_height_value": 1000,
            "timeout_timestamp": 0,
            "acknowledgement_data": [115, 117, 99, 99, 101, 115, 115], // "success"
            "ack_proof": [1, 2, 3, 4], // Mock proof
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Verify acknowledgement stored
    let ack = alice
        .call(contract.id(), "ibc_get_packet_acknowledgement")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "sequence": sequence
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(ack.is_some());
    let ack_value = ack.unwrap();
    let ack_data = ack_value["data"].as_array().unwrap();
    let ack_string = String::from_utf8(ack_data.iter().map(|v| v.as_u64().unwrap() as u8).collect()).unwrap();
    assert_eq!(ack_string, "success");

    // Verify packet commitment removed
    let commitment = alice
        .call(contract.id(), "ibc_get_packet_commitment")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "sequence": sequence
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(commitment.is_none());

    println!("✅ AcknowledgePacket test passed - Acknowledgement stored, commitment removed");
    Ok(())
}

#[tokio::test]
async fn test_ordered_channel_sequence_validation() -> Result<()> {
    // Add small delay to avoid port conflicts
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup ordered channel
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;
    
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 1, // Ordered
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "counterparty_channel_id": "channel-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Try to receive packet with sequence 2 (should fail - expecting 1)
    let result = alice
        .call(contract.id(), "ibc_recv_packet")
        .args_json(json!({
            "sequence": 2,
            "source_port": "transfer",
            "source_channel": "channel-1",
            "destination_port": "transfer",
            "destination_channel": channel_id,
            "data": [72, 101, 108, 108, 111],
            "timeout_height_revision": 1,
            "timeout_height_value": 1000,
            "timeout_timestamp": 0,
            "packet_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?;

    // Should fail with sequence error
    assert!(result.is_failure());

    // Receive packet with correct sequence 1
    alice
        .call(contract.id(), "ibc_recv_packet")
        .args_json(json!({
            "sequence": 1,
            "source_port": "transfer",
            "source_channel": "channel-1",
            "destination_port": "transfer",
            "destination_channel": channel_id,
            "data": [72, 101, 108, 108, 111],
            "timeout_height_revision": 1,
            "timeout_height_value": 1000,
            "timeout_timestamp": 0,
            "packet_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Verify next receive sequence updated
    let next_recv = alice
        .call(contract.id(), "ibc_get_next_sequence_recv")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<u64>()?;

    assert_eq!(next_recv, 2);

    println!("✅ Ordered channel sequence validation test passed");
    Ok(())
}

#[tokio::test]
async fn test_packet_timeout_validation() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup open channel
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;
    
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "counterparty_channel_id": "channel-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Try to receive packet with very low timeout height (should fail)
    let result = alice
        .call(contract.id(), "ibc_recv_packet")
        .args_json(json!({
            "sequence": 1,
            "source_port": "transfer",
            "source_channel": "channel-1",
            "destination_port": "transfer",
            "destination_channel": channel_id,
            "data": [72, 101, 108, 108, 111],
            "timeout_height_revision": 1,
            "timeout_height_value": 1, // Very low timeout
            "timeout_timestamp": 0,
            "packet_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?;

    // Should fail due to timeout
    assert!(result.is_failure());

    println!("✅ Packet timeout validation test passed - Timed out packet rejected");
    Ok(())
}

#[tokio::test]
async fn test_channel_helper_functions() -> Result<()> {
    // Add small delay to avoid port conflicts
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Test acknowledgement helper functions
    let success_ack = alice
        .call(contract.id(), "ibc_create_success_acknowledgement")
        .args_json(json!({
            "result": [115, 117, 99, 99, 101, 115, 115] // "success"
        }))
        .view()
        .await?
        .json::<serde_json::Value>()?;

    let error_ack = alice
        .call(contract.id(), "ibc_create_error_acknowledgement")
        .args_json(json!({
            "error": "error occurred"
        }))
        .view()
        .await?
        .json::<serde_json::Value>()?;

    // Test acknowledgement success checking
    let is_success = alice
        .call(contract.id(), "ibc_is_acknowledgement_success")
        .args_json(json!({
            "ack": success_ack
        }))
        .view()
        .await?
        .json::<bool>()?;

    let is_error = alice
        .call(contract.id(), "ibc_is_acknowledgement_success")
        .args_json(json!({
            "ack": error_ack
        }))
        .view()
        .await?
        .json::<bool>()?;

    assert!(is_success);
    assert!(!is_error);

    // Test packet commitment creation
    let commitment = alice
        .call(contract.id(), "ibc_create_packet_commitment")
        .args_json(json!({
            "data": [1, 2, 3, 4, 5]
        }))
        .view()
        .await?
        .json::<serde_json::Value>()?;

    assert!(commitment["data"].as_array().unwrap().len() > 0);

    // Test timeout height validation
    let is_zero = alice
        .call(contract.id(), "ibc_is_timeout_height_zero")
        .args_json(json!({
            "height_revision": 0,
            "height_value": 0
        }))
        .view()
        .await?
        .json::<bool>()?;

    let is_not_zero = alice
        .call(contract.id(), "ibc_is_timeout_height_zero")
        .args_json(json!({
            "height_revision": 1,
            "height_value": 100
        }))
        .view()
        .await?
        .json::<bool>()?;

    assert!(is_zero);
    assert!(!is_not_zero);

    println!("✅ Channel helper functions test passed");
    Ok(())
}

#[tokio::test]
async fn test_channel_state_management() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup prerequisites
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;

    // Test channel state queries on non-existent channel
    let non_existent = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": "channel-999"
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(non_existent.is_none());

    let is_open = alice
        .call(contract.id(), "ibc_is_channel_open")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": "channel-999"
        }))
        .view()
        .await?
        .json::<bool>()?;

    assert!(!is_open);

    // Create channel and test state progression
    let channel_id = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    // Channel should exist but not be open
    let channel = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?;

    assert!(channel.is_some());
    assert_eq!(channel.unwrap()["state"], "Init");

    let is_open = alice
        .call(contract.id(), "ibc_is_channel_open")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<bool>()?;

    assert!(!is_open);

    // Open the channel
    alice
        .call(contract.id(), "ibc_chan_open_ack")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id,
            "counterparty_channel_id": "channel-1",
            "counterparty_version": "ics20-1",
            "channel_proof": [1, 2, 3, 4],
            "proof_height": 100
        }))
        .transact()
        .await?
        .into_result()?;

    // Channel should now be open
    let is_open = alice
        .call(contract.id(), "ibc_is_channel_open")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": channel_id
        }))
        .view()
        .await?
        .json::<bool>()?;

    assert!(is_open);

    println!("✅ Channel state management test passed");
    Ok(())
}

#[tokio::test]
async fn test_multiple_channels() -> Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let contract = deploy_cosmos_contract(&worker).await?;
    let alice = create_test_account(&worker, "alice").await?;

    // Setup prerequisites
    let client_id = setup_ibc_client(&contract, &alice).await?;
    let connection_id = setup_ibc_connection(&contract, &alice, &client_id).await?;

    // Create multiple channels on different ports
    let transfer_channel = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "transfer",
            "order": 0,
            "connection_hops": [connection_id.clone()],
            "counterparty_port_id": "transfer",
            "version": "ics20-1"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    let custom_channel = alice
        .call(contract.id(), "ibc_chan_open_init")
        .args_json(json!({
            "port_id": "custom-app",
            "order": 1, // Ordered
            "connection_hops": [connection_id],
            "counterparty_port_id": "custom-app",
            "version": "v1.0.0"
        }))
        .transact()
        .await?
        .into_result()?
        .json::<String>()?;

    // Verify different channel IDs
    assert_eq!(transfer_channel, "channel-0");
    assert_eq!(custom_channel, "channel-1");

    // Verify different configurations
    let transfer_chan = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": transfer_channel
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?
        .unwrap();

    let custom_chan = alice
        .call(contract.id(), "ibc_get_channel")
        .args_json(json!({
            "port_id": "custom-app",
            "channel_id": custom_channel
        }))
        .view()
        .await?
        .json::<Option<serde_json::Value>>()?
        .unwrap();

    assert_eq!(transfer_chan["ordering"], "Unordered");
    assert_eq!(transfer_chan["version"], "ics20-1");
    assert_eq!(custom_chan["ordering"], "Ordered");
    assert_eq!(custom_chan["version"], "v1.0.0");

    // Verify isolated sequence tracking
    let transfer_seq = alice
        .call(contract.id(), "ibc_get_next_sequence_send")
        .args_json(json!({
            "port_id": "transfer",
            "channel_id": transfer_channel
        }))
        .view()
        .await?
        .json::<u64>()?;

    let custom_seq = alice
        .call(contract.id(), "ibc_get_next_sequence_send")
        .args_json(json!({
            "port_id": "custom-app",
            "channel_id": custom_channel
        }))
        .view()
        .await?
        .json::<u64>()?;

    assert_eq!(transfer_seq, 1);
    assert_eq!(custom_seq, 1);

    println!("✅ Multiple channels test passed - {} & {}", transfer_channel, custom_channel);
    Ok(())
}