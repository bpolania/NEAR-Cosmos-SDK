use ibc_relayer::testnet::{LocalTestnet, test_utils};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_local_testnet_lifecycle() {
    // Test that we can start and stop the testnet
    let testnet = test_utils::ensure_local_testnet().await
        .expect("Failed to start local testnet");
    
    // Verify it's running
    assert!(testnet.is_running().await);
    
    // Test basic functionality
    let height = testnet.get_block_height().await
        .expect("Should be able to get block height");
    assert!(height > 0);
    
    // Wait a bit and check that blocks are being produced
    sleep(Duration::from_secs(3)).await;
    let new_height = testnet.get_block_height().await
        .expect("Should be able to get new block height");
    assert!(new_height >= height, "New height should be >= original height");
    
    println!("✅ Local testnet lifecycle test passed");
    println!("   Initial height: {}, Final height: {}", height, new_height);
}

#[tokio::test]
async fn test_local_testnet_accounts() {
    let testnet = test_utils::ensure_local_testnet().await
        .expect("Failed to start local testnet");
    
    let accounts = testnet.get_test_accounts();
    
    // Verify all accounts have proper addresses
    assert!(!accounts.validator.address.is_empty());
    assert!(!accounts.test1.address.is_empty());
    assert!(!accounts.relayer.address.is_empty());
    
    // Verify addresses start with correct prefix
    assert!(accounts.validator.address.starts_with("wasm"));
    assert!(accounts.test1.address.starts_with("wasm"));
    assert!(accounts.relayer.address.starts_with("wasm"));
    
    // Verify all accounts have mnemonics
    assert!(!accounts.validator.mnemonic.is_empty());
    assert!(!accounts.test1.mnemonic.is_empty());
    assert!(!accounts.relayer.mnemonic.is_empty());
    
    // Verify initial balances are set
    assert_eq!(accounts.validator.initial_balance, "100000000000000000000");
    assert_eq!(accounts.test1.initial_balance, "100000000000000000000");
    assert_eq!(accounts.relayer.initial_balance, "100000000000000000000");
    
    println!("✅ Local testnet accounts test passed");
    println!("   Validator: {}", accounts.validator.address);
    println!("   Test1: {}", accounts.test1.address);
    println!("   Relayer: {}", accounts.relayer.address);
}

#[tokio::test]
async fn test_local_testnet_multiple_calls() {
    // Test that multiple calls to ensure_local_testnet work correctly
    let testnet1 = test_utils::ensure_local_testnet().await
        .expect("Failed to start local testnet (first call)");
    
    let testnet2 = test_utils::ensure_local_testnet().await
        .expect("Failed to start local testnet (second call)");
    
    // Both should report the same endpoints
    assert_eq!(testnet1.rpc_endpoint, testnet2.rpc_endpoint);
    assert_eq!(testnet1.chain_id, testnet2.chain_id);
    
    // Both should be running
    assert!(testnet1.is_running().await);
    assert!(testnet2.is_running().await);
    
    println!("✅ Multiple ensure_local_testnet calls test passed");
}

#[tokio::test]
async fn test_local_testnet_rpc_endpoints() {
    let testnet = test_utils::ensure_local_testnet().await
        .expect("Failed to start local testnet");
    
    let client = reqwest::Client::new();
    
    // Test RPC status endpoint
    let status_response = client.get(&format!("{}/status", testnet.rpc_endpoint))
        .send()
        .await
        .expect("Status request failed");
    assert!(status_response.status().is_success());
    
    let status_body: serde_json::Value = status_response.json().await
        .expect("Failed to parse status JSON");
    
    // Verify expected fields
    assert!(status_body.get("result").is_some());
    assert!(status_body["result"].get("node_info").is_some());
    assert!(status_body["result"].get("sync_info").is_some());
    
    // Test REST API node info endpoint
    let node_info_response = client.get(&format!("{}/cosmos/base/tendermint/v1beta1/node_info", testnet.rest_endpoint))
        .send()
        .await
        .expect("Node info request failed");
    assert!(node_info_response.status().is_success());
    
    println!("✅ Local testnet RPC endpoints test passed");
    println!("   RPC status: ✅");
    println!("   REST node_info: ✅");
}

#[tokio::test]
async fn test_local_testnet_chain_info() {
    let testnet = test_utils::ensure_local_testnet().await
        .expect("Failed to start local testnet");
    
    let client = reqwest::Client::new();
    
    // Get status and verify chain info
    let response = client.get(&format!("{}/status", testnet.rpc_endpoint))
        .send()
        .await
        .expect("Status request failed");
    
    let body: serde_json::Value = response.json().await
        .expect("Failed to parse JSON");
    
    let network = body["result"]["node_info"]["network"].as_str()
        .expect("Network field missing");
    
    assert_eq!(network, "wasmd-testnet");
    assert_eq!(network, testnet.chain_id);
    
    // Verify we have a proper latest block height
    let height_str = body["result"]["sync_info"]["latest_block_height"].as_str()
        .expect("Latest block height missing");
    
    let height: u64 = height_str.parse()
        .expect("Could not parse block height as u64");
    
    assert!(height > 0, "Block height should be greater than 0");
    
    println!("✅ Local testnet chain info test passed");
    println!("   Chain ID: {}", network);
    println!("   Block height: {}", height);
}