// Example demonstrating Cosmos transaction signing and broadcasting
// This shows how to configure a Cosmos chain with real cryptographic signing

use ibc_relayer::chains::cosmos_minimal::CosmosChain;
use ibc_relayer::config::{ChainConfig, ChainSpecificConfig};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔐 Cosmos Transaction Signing Example");
    
    // 1. Create chain configuration for Cosmos testnet
    let config = ChainConfig {
        chain_id: "cosmoshub-testnet".to_string(),
        chain_type: "cosmos".to_string(),
        rpc_endpoint: "https://rpc.testnet.cosmos.network".to_string(),
        ws_endpoint: None,
        config: ChainSpecificConfig::Cosmos {
            address_prefix: "cosmos".to_string(),
            gas_price: "0.025uatom".to_string(),
            trust_threshold: "1/3".to_string(),
            trusting_period_hours: 336,
            signer_key: None,
        },
    };
    
    // 2. Create Cosmos chain instance
    let mut cosmos_chain = CosmosChain::new(&config)?;
    println!("✅ Created Cosmos chain instance for {}", config.chain_id);
    
    // 3. Configure account with private key (DEMO ONLY - use secure key management in production)
    let demo_private_key = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    let demo_address = "cosmos1demo123address456789";
    
    println!("⚠️  This is a DEMO - Never hardcode private keys in production!");
    
    // Configure account for signing (this would fail without real account info)
    match cosmos_chain.configure_account_with_key(
        demo_address.to_string(),
        demo_private_key.to_string()
    ).await {
        Ok(()) => {
            println!("🔑 Account configured with signing key");
            
            // 4. Build and sign a transaction
            let ibc_recv_message = vec![json!({
                "@type": "/ibc.core.channel.v1.MsgRecvPacket",
                "packet": {
                    "sequence": "1",
                    "source_port": "transfer",
                    "source_channel": "channel-0", 
                    "destination_port": "transfer",
                    "destination_channel": "channel-1",
                    "data": "eyJhbW91bnQiOiIxMDAwIiwiZGVub20iOiJ1YXRvbSJ9", // base64 encoded JSON
                    "timeout_height": {
                        "revision_number": "0",
                        "revision_height": "1000"
                    }
                },
                "proof_commitment": "cHJvb2ZfZGF0YQ==", // base64 encoded proof
                "proof_height": {
                    "revision_number": "0", 
                    "revision_height": "999"
                },
                "signer": demo_address
            })];
            
            // Attempt to build and broadcast transaction
            match cosmos_chain.build_and_broadcast_tx(
                ibc_recv_message,
                "IBC packet relay from NEAR".to_string(),
                200_000, // gas limit
            ).await {
                Ok(tx_hash) => {
                    println!("✅ Transaction signed and broadcast: {}", tx_hash);
                }
                Err(e) => {
                    println!("⚠️  Transaction broadcast failed (expected for demo): {}", e);
                    println!("   This is expected - demo account doesn't exist on chain");
                }
            }
        }
        Err(e) => {
            println!("⚠️  Account configuration failed (expected for demo): {}", e);
            println!("   In production, use real account with on-chain balance");
        }
    }
    
    // 5. Demonstrate key derivation
    println!("\n🔧 Key Derivation Demo:");
    match cosmos_chain.derive_public_key(&hex::decode(demo_private_key)?) {
        Ok(public_key) => {
            println!("   Private key: {}...", &demo_private_key[..16]);
            println!("   Public key:  {} ({} bytes)", hex::encode(&public_key), public_key.len());
            println!("   Key type:    secp256k1 compressed");
        }
        Err(e) => {
            println!("   Key derivation failed: {}", e);
        }
    }
    
    // 6. Production usage guidelines
    println!("\n📋 Production Usage Guidelines:");
    println!("   1. Never hardcode private keys - use secure key management");
    println!("   2. Store keys in encrypted keystores or hardware security modules");
    println!("   3. Use environment variables or config files for addresses");
    println!("   4. Always verify transaction success before proceeding");
    println!("   5. Implement proper error handling and retry logic");
    
    println!("\n🎯 Example completed successfully!");
    Ok(())
}