use ibc_relayer::chains::cosmos_minimal::CosmosChain;
use ibc_relayer::chains::Chain;
use ibc_relayer::config::{ChainConfig, ChainSpecificConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧪 Testing Cosmos Provider Testnet Connection");
    
    // Create config for provider testnet
    let config = ChainConfig {
        chain_id: "provider".to_string(),
        chain_type: "cosmos".to_string(),
        rpc_endpoint: "https://rest.provider-sentry-01.ics-testnet.polypore.xyz".to_string(),
        ws_endpoint: Some("wss://rpc.provider-sentry-01.ics-testnet.polypore.xyz/websocket".to_string()),
        config: ChainSpecificConfig::Cosmos {
            address_prefix: "cosmos".to_string(),
            gas_price: "0.025uatom".to_string(),
            signer_key: None,
            trust_threshold: "1/3".to_string(),
            trusting_period_hours: 336,
        },
    };
    
    // Create Cosmos chain instance
    let cosmos_chain = CosmosChain::new(&config)?;
    
    println!("✅ Created Cosmos chain instance");
    println!("   Chain ID: {}", cosmos_chain.chain_id().await);
    println!("   RPC Endpoint: {}", config.rpc_endpoint);
    
    // Test basic connectivity
    match cosmos_chain.get_latest_height().await {
        Ok(height) => {
            println!("✅ Successfully connected to provider testnet");
            println!("   Latest height: {}", height);
        }
        Err(e) => {
            println!("❌ Failed to connect to provider testnet: {}", e);
            println!("   This might be expected if the testnet is down or endpoint changed");
        }
    }
    
    // Test status query
    match cosmos_chain.query_status().await {
        Ok(status) => {
            println!("✅ Chain status:");
            println!("   Chain ID: {}", status.chain_id);
            println!("   Latest Block: {}", status.latest_block_height);
            println!("   Latest Time: {}", status.latest_block_time);
        }
        Err(e) => {
            println!("❌ Failed to query status: {}", e);
        }
    }
    
    println!("\n🔍 Testing with alternative endpoints...");
    
    // Try alternative RPC endpoint
    let alt_config = ChainConfig {
        chain_id: "provider".to_string(), 
        chain_type: "cosmos".to_string(),
        rpc_endpoint: "https://rpc.provider-sentry-01.ics-testnet.polypore.xyz".to_string(),
        ws_endpoint: Some("wss://rpc.provider-sentry-01.ics-testnet.polypore.xyz/websocket".to_string()),
        config: ChainSpecificConfig::Cosmos {
            address_prefix: "cosmos".to_string(),
            gas_price: "0.025uatom".to_string(),
            signer_key: None,
            trust_threshold: "1/3".to_string(),
            trusting_period_hours: 336,
        },
    };
    
    let alt_cosmos_chain = CosmosChain::new(&alt_config)?;
    
    match alt_cosmos_chain.get_latest_height().await {
        Ok(height) => {
            println!("✅ Alternative RPC endpoint working");
            println!("   Latest height: {}", height);
        }
        Err(e) => {
            println!("❌ Alternative RPC also failed: {}", e);
        }
    }
    
    println!("\n🎯 Cosmos connectivity test completed");
    Ok(())
}