// Key management CLI for IBC relayer
// Provides secure key storage, retrieval, and management capabilities

use clap::{Parser, Subcommand};
use ibc_relayer::keystore::{KeyManager, KeyManagerConfig, KeyEntry};
use ibc_relayer::keystore::cosmos::CosmosKey;
use ibc_relayer::keystore::near::NearKey;
use std::io::{self, Write};

#[derive(Parser)]
#[clap(name = "key-manager")]
#[clap(about = "IBC Relayer Key Management Tool")]
#[clap(version)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new key to the keystore
    Add {
        /// Chain ID (e.g., "cosmoshub-testnet", "near-testnet")
        chain_id: String,
        /// Key type ("cosmos" or "near")
        #[clap(short, long)]
        key_type: String,
        /// Import from private key (hex for Cosmos, secret key string for NEAR)
        #[clap(short, long)]
        private_key: Option<String>,
        /// Account address or ID
        #[clap(short, long)]
        address: Option<String>,
    },
    /// List all keys in the keystore
    List,
    /// Show details of a specific key
    Show {
        /// Chain ID
        chain_id: String,
    },
    /// Remove a key from the keystore
    Remove {
        /// Chain ID
        chain_id: String,
    },
    /// Export a key (for backup)
    Export {
        /// Chain ID
        chain_id: String,
    },
    /// Test key loading from environment variables
    TestEnv {
        /// Chain ID to test
        chain_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize key manager with default config
    let config = KeyManagerConfig::default();
    let mut key_manager = KeyManager::new(config)?;
    
    match cli.command {
        Commands::Add { chain_id, key_type, private_key, address } => {
            add_key(&mut key_manager, &chain_id, &key_type, private_key, address).await?;
        }
        Commands::List => {
            list_keys(&key_manager).await?;
        }
        Commands::Show { chain_id } => {
            show_key(&mut key_manager, &chain_id).await?;
        }
        Commands::Remove { chain_id } => {
            remove_key(&mut key_manager, &chain_id).await?;
        }
        Commands::Export { chain_id } => {
            export_key(&key_manager, &chain_id).await?;
        }
        Commands::TestEnv { chain_id } => {
            test_env_key(&mut key_manager, &chain_id).await?;
        }
    }
    
    Ok(())
}

async fn add_key(
    key_manager: &mut KeyManager,
    chain_id: &str,
    key_type: &str,
    private_key: Option<String>,
    address: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Adding key for chain: {}", chain_id);
    
    let key_entry = match key_type.to_lowercase().as_str() {
        "cosmos" => {
            let private_key = if let Some(pk) = private_key {
                pk
            } else {
                print!("Enter private key (hex): ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_string()
            };
            
            let address = if let Some(addr) = address {
                addr
            } else {
                print!("Enter Cosmos address: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_string()
            };
            
            // Create Cosmos key
            let cosmos_key = CosmosKey::from_env_string(&format!("{}:{}", address, private_key))?;
            KeyEntry::Cosmos(cosmos_key)
        }
        "near" => {
            let secret_key = if let Some(sk) = private_key {
                sk
            } else {
                print!("Enter secret key (ed25519:base58): ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_string()
            };
            
            let account_id = if let Some(addr) = address {
                addr
            } else {
                print!("Enter NEAR account ID: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().to_string()
            };
            
            // Create NEAR key
            let near_key = NearKey::from_env_string(&format!("{}:{}", account_id, secret_key))?;
            KeyEntry::Near(near_key)
        }
        _ => {
            return Err(format!("Unsupported key type: {}. Use 'cosmos' or 'near'", key_type).into());
        }
    };
    
    // Get password for encryption
    print!("Enter password to encrypt the key: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;
    
    // Store the key
    key_manager.store_key(chain_id, key_entry, &password).await?;
    
    println!("✅ Key added successfully for chain: {}", chain_id);
    Ok(())
}

async fn list_keys(key_manager: &KeyManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Available keys:");
    
    let keys = key_manager.list_keys().await?;
    if keys.is_empty() {
        println!("No keys found in keystore.");
        return Ok(());
    }
    
    for chain_id in keys {
        println!("  • {}", chain_id);
    }
    
    Ok(())
}

async fn show_key(
    key_manager: &mut KeyManager,
    chain_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Key details for chain: {}", chain_id);
    
    // Try to load from environment first
    match key_manager.load_key(chain_id).await {
        Ok(_) => {
            if let Ok(address) = key_manager.get_address(chain_id) {
                println!("  Address: {}", address);
                println!("  Source: environment variable or keystore");
            }
        }
        Err(e) => {
            println!("❌ Failed to load key: {}", e);
            println!("💡 Try setting environment variable: RELAYER_KEY_{}", chain_id.to_uppercase());
        }
    }
    
    Ok(())
}

async fn remove_key(
    key_manager: &mut KeyManager,
    chain_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    print!("Are you sure you want to remove key for {}? (y/N): ", chain_id);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Cancelled.");
        return Ok(());
    }
    
    key_manager.remove_key(chain_id).await?;
    println!("✅ Key removed for chain: {}", chain_id);
    Ok(())
}

async fn export_key(
    key_manager: &KeyManager,
    chain_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠️  WARNING: This will display your private key in plain text!");
    print!("Continue? (y/N): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("Cancelled.");
        return Ok(());
    }
    
    match key_manager.export_key(chain_id) {
        Ok(exported) => {
            println!("🔑 Exported key for {}:", chain_id);
            println!("{}", exported);
            println!("⚠️  Keep this safe and never share it!");
        }
        Err(e) => {
            println!("❌ Failed to export key: {}", e);
        }
    }
    
    Ok(())
}

async fn test_env_key(
    key_manager: &mut KeyManager,
    chain_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing environment variable key loading for: {}", chain_id);
    println!("Expected env var: RELAYER_KEY_{}", chain_id.to_uppercase());
    
    match key_manager.load_key(chain_id).await {
        Ok(_) => {
            if let Ok(address) = key_manager.get_address(chain_id) {
                println!("✅ Successfully loaded key from environment");
                println!("   Address: {}", address);
            } else {
                println!("⚠️  Key loaded but address extraction failed");
            }
        }
        Err(e) => {
            println!("❌ Failed to load key from environment: {}", e);
            println!("💡 Set the environment variable:");
            println!("   export RELAYER_KEY_{}=<your_key_data>", chain_id.to_uppercase());
            println!("   For Cosmos: address:private_key_hex");
            println!("   For NEAR: account.near:ed25519:secret_key");
        }
    }
    
    Ok(())
}