use std::env;
use tokio::time::{timeout, Duration};

use ibc_relayer::config::RelayerConfig;
use ibc_relayer::keystore::{KeyManager, KeyManagerConfig};

#[tokio::test]
async fn test_testnet_configuration_parsing() {
    // Load the testnet configuration
    let config = RelayerConfig::load("config/relayer.toml").expect("Failed to load testnet config");
    
    // Verify NEAR testnet configuration
    let near_config = config.chains.get("near-testnet").expect("NEAR testnet config missing");
    assert_eq!(near_config.chain_id, "near-testnet");
    assert_eq!(near_config.rpc_endpoint, "https://rpc.testnet.near.org");
    
    // Verify Cosmos testnet configuration  
    let cosmos_config = config.chains.get("cosmoshub-testnet").expect("Cosmos testnet config missing");
    assert_eq!(cosmos_config.chain_id, "provider");
    assert_eq!(cosmos_config.rpc_endpoint, "https://rest.provider-sentry-01.ics-testnet.polypore.xyz");
}

#[tokio::test]
#[ignore = "Requires live testnet connection"]
async fn test_near_testnet_connectivity() {
    let client = reqwest::Client::new();
    
    // Test NEAR testnet RPC connectivity
    let response = timeout(
        Duration::from_secs(10),
        client.get("https://rpc.testnet.near.org/status").send()
    ).await.expect("Request timed out").expect("Failed to connect to NEAR testnet");
    
    assert!(response.status().is_success());
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["chain_id"], "testnet");
    assert!(body["sync_info"]["latest_block_height"].as_u64().unwrap() > 0);
}

#[tokio::test]
#[ignore = "Requires live testnet connection"]
async fn test_cosmos_testnet_connectivity() {
    let client = reqwest::Client::new();
    
    // Test Cosmos provider testnet RPC connectivity
    let response = timeout(
        Duration::from_secs(10),
        client.get("https://rpc.provider-sentry-01.ics-testnet.polypore.xyz/status").send()
    ).await.expect("Request timed out").expect("Failed to connect to Cosmos testnet");
    
    assert!(response.status().is_success());
    
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["result"]["node_info"]["network"], "provider");
    assert!(body["result"]["sync_info"]["latest_block_height"].as_str().unwrap().parse::<u64>().unwrap() > 0);
}

#[tokio::test]
async fn test_environment_key_loading() {
    // Set test environment variables
    let test_cosmos_key = "cosmos1test:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let test_near_key = "test.testnet:ed25519:5K8HtSNHQDFvEpALHMy4QN9CvZaT6Q4MpX2YmRe3JdKt";
    
    env::set_var("RELAYER_KEY_PROVIDER", test_cosmos_key);
    env::set_var("RELAYER_KEY_NEAR_TESTNET", test_near_key);
    
    // Test key manager loading
    let config = KeyManagerConfig::default();
    let mut key_manager = KeyManager::new(config).expect("Failed to create key manager");
    
    
    // Test Cosmos key loading
    let cosmos_result = key_manager.load_key("provider").await;
    assert!(cosmos_result.is_ok(), "Failed to load Cosmos key from environment: {:?}", cosmos_result.err());
    
    // Test NEAR key loading  
    let near_result = key_manager.load_key("near-testnet").await;
    assert!(near_result.is_ok(), "Failed to load NEAR key from environment: {:?}", near_result.err());
    
    // Clean up environment variables
    env::remove_var("RELAYER_KEY_PROVIDER");
    env::remove_var("RELAYER_KEY_NEAR_TESTNET");
}

#[tokio::test]
#[ignore = "Requires live testnet connection"]
async fn test_testnet_deployment_readiness() {
    // This test verifies that all components needed for testnet deployment are ready
    
    // 1. Configuration can be loaded
    let config = RelayerConfig::load("config/relayer.toml").expect("Config loading failed");
    assert!(config.chains.len() >= 2, "Not enough chains configured");
    
    // 2. Both testnet endpoints are reachable
    let client = reqwest::Client::new();
    
    // Check NEAR testnet
    let near_response = timeout(
        Duration::from_secs(5),
        client.get("https://rpc.testnet.near.org/status").send()
    ).await;
    assert!(near_response.is_ok(), "NEAR testnet unreachable");
    
    // Check Cosmos testnet
    let cosmos_response = timeout(
        Duration::from_secs(5), 
        client.get("https://rpc.provider-sentry-01.ics-testnet.polypore.xyz/status").send()
    ).await;
    assert!(cosmos_response.is_ok(), "Cosmos testnet unreachable");
    
    // 3. Key manager can be initialized
    let key_config = KeyManagerConfig::default();
    let key_manager = KeyManager::new(key_config);
    assert!(key_manager.is_ok(), "Key manager initialization failed");
    
    println!("✅ Testnet deployment readiness verified");
}

#[tokio::test]
#[ignore = "Requires specific testnet key format"] 
async fn test_real_testnet_key_format() {
    // Test with the actual generated testnet key format
    let test_key = "cosmos162ca2a24f0d288439231d29170a101e554b7e6:d600357797a65160742b73279fb55f55faf83258f841e8411d5503b95f079791";
    
    env::set_var("RELAYER_KEY_PROVIDER", test_key);
    
    let config = KeyManagerConfig::default();
    let mut key_manager = KeyManager::new(config).expect("Key manager creation failed");
    
    let result = key_manager.load_key("provider").await;
    assert!(result.is_ok(), "Failed to load real testnet key format: {:?}", result.err());
    
    // Verify we can get the address
    let address = key_manager.get_address("provider");
    assert!(address.is_ok(), "Failed to get address from testnet key");
    assert_eq!(address.unwrap(), "cosmos162ca2a24f0d288439231d29170a101e554b7e6");
    
    env::remove_var("RELAYER_KEY_PROVIDER");
}

#[cfg(test)]
mod testnet_scripts_tests {
    use std::process::Command;
    
    #[test]
    fn test_setup_script_exists() {
        let script_path = "scripts/setup_testnet.sh";
        assert!(std::path::Path::new(script_path).exists(), "Setup script missing");
        
        // Verify script is executable
        let metadata = std::fs::metadata(script_path).expect("Cannot read script metadata");
        let permissions = metadata.permissions();
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert!(permissions.mode() & 0o111 != 0, "Script is not executable");
        }
    }
    
    #[test]
    fn test_key_generation_script_exists() {
        let script_path = "scripts/generate_cosmos_key.sh";
        assert!(std::path::Path::new(script_path).exists(), "Key generation script missing");
    }
    
    #[test]
    fn test_scripts_syntax() {
        // Test that bash scripts have valid syntax
        let setup_result = Command::new("bash")
            .args(["-n", "scripts/setup_testnet.sh"])
            .output()
            .expect("Failed to check setup script syntax");
        
        assert!(setup_result.status.success(), 
            "Setup script has syntax errors: {}", 
            String::from_utf8_lossy(&setup_result.stderr));
            
        let keygen_result = Command::new("bash")
            .args(["-n", "scripts/generate_cosmos_key.sh"])
            .output()
            .expect("Failed to check key generation script syntax");
            
        assert!(keygen_result.status.success(),
            "Key generation script has syntax errors: {}",
            String::from_utf8_lossy(&keygen_result.stderr));
    }
}