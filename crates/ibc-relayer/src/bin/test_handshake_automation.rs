use ibc_relayer::chains::{Chain, near_simple::NearChain, cosmos_minimal::CosmosChain};
use ibc_relayer::config::{ChainConfig, ChainSpecificConfig};
use ibc_relayer::relay::handshake::{ConnectionHandshake, ChannelHandshake, HandshakeCoordinator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧪 Testing IBC Handshake Automation");
    println!("===================================");

    // Create mock chain configurations
    let near_config = ChainConfig {
        chain_id: "near-testnet".to_string(),
        chain_type: "near".to_string(),
        rpc_endpoint: "https://rpc.testnet.near.org".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Near {
            contract_id: "cosmos-sdk-demo.testnet".to_string(),
            signer_account_id: "cuteharbor3573.testnet".to_string(),
            network_id: "testnet".to_string(),
            private_key: None,
        },
    };

    let cosmos_config = ChainConfig {
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

    // Create chain instances
    println!("🔧 Creating chain instances...");
    let near_chain: Box<dyn Chain> = Box::new(
        NearChain::new(&near_config)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?
    );
    let cosmos_chain: Box<dyn Chain> = Box::new(CosmosChain::new(&cosmos_config)?);

    println!("✅ NEAR Chain: {}", near_chain.chain_id().await);
    println!("✅ Cosmos Chain: {}", cosmos_chain.chain_id().await);

    // Create handshake coordinator
    let mut coordinator = HandshakeCoordinator::new();

    println!("\n🤝 Creating Connection Handshake...");
    
    // Create connection handshake
    let connection_handshake = ConnectionHandshake::new(
        near_chain,
        cosmos_chain,
        "connection-0".to_string(),
        "07-tendermint-0".to_string(), // NEAR's client for Cosmos
        "07-near-0".to_string(),       // Cosmos's client for NEAR (hypothetical)
    );

    coordinator.register_connection_handshake("connection-0".to_string(), connection_handshake);

    // Create mock chains for channel handshake
    let near_chain2: Box<dyn Chain> = Box::new(
        NearChain::new(&near_config)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?
    );
    let cosmos_chain2: Box<dyn Chain> = Box::new(CosmosChain::new(&cosmos_config)?);

    println!("🤝 Creating Channel Handshake...");
    
    // Create channel handshake
    let channel_handshake = ChannelHandshake::new(
        near_chain2,
        cosmos_chain2,
        "transfer".to_string(),    // Source port
        "channel-0".to_string(),   // Source channel
        "transfer".to_string(),    // Destination port
        "connection-0".to_string(), // Connection to use
    );

    coordinator.register_channel_handshake("transfer/channel-0".to_string(), channel_handshake);

    // Show initial status
    let status = coordinator.get_status();
    println!("\n📊 Initial Status:");
    println!("   Pending connections: {}", status.pending_connections);
    println!("   Pending channels: {}", status.pending_channels);

    println!("\n🚀 Processing Handshakes...");
    println!("============================");

    // Process all handshakes
    match coordinator.process_handshakes().await {
        Ok(_) => {
            println!("\n✅ Handshake processing completed");
        }
        Err(e) => {
            println!("\n❌ Handshake processing failed: {}", e);
        }
    }

    // Show final status
    let final_status = coordinator.get_status();
    println!("\n📊 Final Status:");
    println!("   Pending connections: {}", final_status.pending_connections);
    println!("   Pending channels: {}", final_status.pending_channels);

    println!("\n💡 Notes:");
    println!("- This demonstrates the handshake automation framework");
    println!("- Real handshakes require both chains to have corresponding clients");
    println!("- Proof generation and verification will be implemented when needed");
    println!("- The framework is ready for production use once infrastructure is complete");

    Ok(())
}