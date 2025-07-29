// Example demonstrating the secure key management system
// Shows encrypted keystore, environment variable loading, and secure practices

use ibc_relayer::keystore::{KeyManager, KeyManagerConfig, KeyEntry};
use ibc_relayer::keystore::cosmos::CosmosKey;
use ibc_relayer::keystore::near::NearKey;
use std::env;
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Key Management System Example");
    println!("This example demonstrates secure key storage and retrieval");
    
    // 1. Create a temporary directory for this demo
    let temp_dir = tempdir()?;
    let config = KeyManagerConfig {
        keystore_dir: temp_dir.path().to_path_buf(),
        allow_env_keys: true,
        env_prefix: "DEMO_KEY_".to_string(),
        kdf_iterations: 10_000,
    };
    
    let mut key_manager = KeyManager::new(config)?;
    println!("✅ Created key manager with temp directory: {:?}", temp_dir.path());
    
    // 2. Demonstrate Cosmos key creation and storage
    println!("\n📋 Creating and storing Cosmos key...");
    
    let demo_cosmos_key = CosmosKey::from_private_key(
        hex::decode("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")?,
        "cosmos"
    )?;
    
    println!("   Generated address: {}", demo_cosmos_key.address);
    println!("   Public key: {}...", &demo_cosmos_key.public_key_hex()[..20]);
    
    let cosmos_key_entry = KeyEntry::Cosmos(demo_cosmos_key);
    key_manager.store_key("cosmoshub-testnet", cosmos_key_entry, "secure_password").await?;
    println!("✅ Cosmos key stored with encryption");
    
    // 3. Demonstrate NEAR key creation and storage
    println!("\n📋 Creating and storing NEAR key...");
    
    let demo_near_key = NearKey::from_secret_key(
        "demo.testnet".to_string(),
        "ed25519:demo_secret_key_placeholder"
    )?;
    
    println!("   Account ID: {}", demo_near_key.account_id);
    println!("   Key type: {}", demo_near_key.key_type);
    
    let near_key_entry = KeyEntry::Near(demo_near_key);
    key_manager.store_key("near-testnet", near_key_entry, "another_password").await?;
    println!("✅ NEAR key stored with encryption");
    
    // 4. List all stored keys
    println!("\n📋 Listing all stored keys:");
    let keys = key_manager.list_keys().await?;
    for key in &keys {
        println!("   • {}", key);
    }
    println!("✅ Found {} keys in keystore", keys.len());
    
    // 5. Demonstrate environment variable key loading
    println!("\n🌍 Testing environment variable key loading:");
    
    // Set environment variables for demo
    env::set_var("DEMO_KEY_COSMOS_MAINNET", "cosmos1demo:deadbeefcafe1234567890abcdef1234567890abcdef1234567890abcdef");
    env::set_var("DEMO_KEY_NEAR_MAINNET", "demo.near:ed25519:env_secret_key_placeholder");
    
    // Try loading from environment
    match key_manager.load_key("cosmos-mainnet").await {
        Ok(_) => {
            if let Ok(address) = key_manager.get_address("cosmos-mainnet") {
                println!("✅ Loaded Cosmos key from environment: {}", address);
            }
        }
        Err(e) => println!("⚠️  Failed to load Cosmos key from env: {}", e),
    }
    
    match key_manager.load_key("near-mainnet").await {
        Ok(_) => {
            if let Ok(address) = key_manager.get_address("near-mainnet") {
                println!("✅ Loaded NEAR key from environment: {}", address);
            }
        }
        Err(e) => println!("⚠️  Failed to load NEAR key from env: {}", e),
    }
    
    // 6. Demonstrate key validation
    println!("\n🔍 Validating keys:");
    
    // Load and validate Cosmos key
    if let Ok(KeyEntry::Cosmos(cosmos_key)) = key_manager.load_key("cosmoshub-testnet").await {
        match cosmos_key.validate() {
            Ok(()) => println!("✅ Cosmos key validation passed"),
            Err(e) => println!("❌ Cosmos key validation failed: {}", e),
        }
    }
    
    // Load and validate NEAR key  
    if let Ok(KeyEntry::Near(near_key)) = key_manager.load_key("near-testnet").await {
        match near_key.validate() {
            Ok(()) => println!("✅ NEAR key validation passed"),
            Err(e) => println!("❌ NEAR key validation failed: {}", e),
        }
    }
    
    // 7. Security best practices demonstration
    println!("\n🛡️  Security Best Practices:");
    println!("   ✅ Keys are encrypted at rest with AES-256-GCM");
    println!("   ✅ Passwords are hashed with Argon2 (industry standard)");
    println!("   ✅ Environment variables provide secure runtime loading");
    println!("   ✅ Private keys are never logged or displayed in plaintext");
    println!("   ✅ Key validation ensures cryptographic integrity");
    
    // 8. Production usage guidelines
    println!("\n📋 Production Usage Guidelines:");
    println!("   1. Store keystore files in secure locations (e.g., ~/.relayer/keys)");
    println!("   2. Use strong passwords for key encryption");
    println!("   3. Set restrictive file permissions (600) on keystore files");
    println!("   4. Use environment variables for keys in containerized deployments");
    println!("   5. Regularly rotate keys and passwords");
    println!("   6. Backup encrypted keystore files securely");
    println!("   7. Use hardware security modules (HSMs) for production mainnet keys");
    
    // 9. CLI tool demonstration
    println!("\n🛠️  CLI Tool Usage:");
    println!("   Add key:    cargo run --bin key-manager add <chain-id> --key-type cosmos");
    println!("   List keys:  cargo run --bin key-manager list");
    println!("   Show key:   cargo run --bin key-manager show <chain-id>");
    println!("   Test env:   cargo run --bin key-manager test-env <chain-id>");
    println!("   Export:     cargo run --bin key-manager export <chain-id>");
    
    // Clean up environment variables
    env::remove_var("DEMO_KEY_COSMOS_MAINNET");
    env::remove_var("DEMO_KEY_NEAR_MAINNET");
    
    println!("\n🎯 Key management example completed successfully!");
    println!("   Temporary keystore will be cleaned up automatically.");
    
    Ok(())
}