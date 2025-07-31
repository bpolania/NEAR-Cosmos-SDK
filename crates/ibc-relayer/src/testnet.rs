use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use anyhow::{Result, anyhow};

/// Local wasmd testnet manager for integration tests
pub struct LocalTestnet {
    pub chain_id: String,
    pub rpc_endpoint: String,
    pub rest_endpoint: String,
    pub grpc_endpoint: String,
}

impl Default for LocalTestnet {
    fn default() -> Self {
        Self {
            chain_id: "wasmd-testnet".to_string(),
            rpc_endpoint: "http://localhost:26657".to_string(),
            rest_endpoint: "http://localhost:1317".to_string(),
            grpc_endpoint: "http://localhost:9090".to_string(),
        }
    }
}

impl LocalTestnet {
    /// Start the local wasmd testnet using Docker Compose
    pub async fn start() -> Result<Self> {
        let testnet = Self::default();
        
        println!("🚀 Starting local wasmd testnet...");
        
        // Check if Docker is available
        let output = Command::new("docker")
            .args(&["--version"])
            .output();
            
        if output.is_err() {
            return Err(anyhow!("Docker is not available. Please install Docker to run local testnet tests."));
        }
        
        // Start the testnet using our helper script
        let output = Command::new("./scripts/local-testnet.sh")
            .args(&["start"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to start local testnet: {}", stderr));
        }
        
        // Wait for the testnet to become available
        testnet.wait_for_ready().await?;
        
        println!("✅ Local wasmd testnet is ready!");
        println!("   RPC: {}", testnet.rpc_endpoint);
        println!("   REST: {}", testnet.rest_endpoint);
        println!("   Chain ID: {}", testnet.chain_id);
        
        Ok(testnet)
    }
    
    /// Stop the local wasmd testnet
    pub async fn stop(&self) -> Result<()> {
        println!("🛑 Stopping local wasmd testnet...");
        
        let output = Command::new("./scripts/local-testnet.sh")
            .args(&["stop"])
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to stop local testnet: {}", stderr));
        }
        
        println!("✅ Local wasmd testnet stopped");
        Ok(())
    }
    
    /// Wait for the testnet to become ready
    async fn wait_for_ready(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let start_time = Instant::now();
        let timeout = Duration::from_secs(60);
        
        while start_time.elapsed() < timeout {
            match client.get(&format!("{}/status", self.rpc_endpoint)).send().await {
                Ok(response) if response.status().is_success() => {
                    // Additional check - make sure we can parse the response
                    if let Ok(body) = response.json::<serde_json::Value>().await {
                        if body.get("result").is_some() {
                            return Ok(());
                        }
                    }
                }
                _ => {}
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        Err(anyhow!("Local testnet did not become ready within {} seconds", timeout.as_secs()))
    }
    
    /// Check if the local testnet is running
    pub async fn is_running(&self) -> bool {
        let client = reqwest::Client::new();
        
        match client.get(&format!("{}/status", self.rpc_endpoint))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
    
    /// Get the current block height
    pub async fn get_block_height(&self) -> Result<u64> {
        let client = reqwest::Client::new();
        
        let response = client.get(&format!("{}/status", self.rpc_endpoint))
            .send()
            .await?;
            
        let body: serde_json::Value = response.json().await?;
        
        let height_str = body
            .get("result")
            .and_then(|r| r.get("sync_info"))
            .and_then(|s| s.get("latest_block_height"))
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow!("Could not parse block height from response"))?;
            
        height_str.parse::<u64>()
            .map_err(|e| anyhow!("Could not parse block height as u64: {}", e))
    }
    
    /// Get test account information
    pub fn get_test_accounts(&self) -> TestAccounts {
        TestAccounts::default()
    }
}

/// Pre-configured test accounts with known addresses and balances
#[derive(Debug, Clone)]
pub struct TestAccounts {
    pub validator: TestAccount,
    pub test1: TestAccount,
    pub relayer: TestAccount,
}

#[derive(Debug, Clone)]
pub struct TestAccount {
    pub address: String,
    pub mnemonic: String,
    pub initial_balance: String,
}

impl Default for TestAccounts {
    fn default() -> Self {
        Self {
            validator: TestAccount {
                address: "wasm1qqxqfzagzm7r8m4zq9gfmk9g5k7nfcasg8jhxm".to_string(),
                mnemonic: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art".to_string(),
                initial_balance: "100000000000000000000".to_string(),
            },
            test1: TestAccount {
                address: "wasm1qy5ldxnqc5x2sffm4m4fl2nzm6q6lzfcx8x4sy".to_string(),
                mnemonic: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string(),
                initial_balance: "100000000000000000000".to_string(),
            },
            relayer: TestAccount {
                address: "wasm1qvw7ks35q7r2qm4m7w7k2lr8m8x6sl2cy8d0mn".to_string(),
                mnemonic: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon above".to_string(),
                initial_balance: "100000000000000000000".to_string(),
            },
        }
    }
}

/// Test utilities for working with the local testnet
pub mod test_utils {
    use super::*;
    
    /// Ensure local testnet is running for tests
    pub async fn ensure_local_testnet() -> Result<LocalTestnet> {
        let testnet = LocalTestnet::default();
        
        if testnet.is_running().await {
            println!("✅ Local testnet already running");
            return Ok(testnet);
        }
        
        println!("🚀 Starting local testnet for tests...");
        LocalTestnet::start().await
    }
    
    /// Cleanup function for tests
    pub async fn cleanup_local_testnet(testnet: &LocalTestnet) -> Result<()> {
        testnet.stop().await
    }
}